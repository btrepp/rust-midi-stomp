#![no_std]
#![no_main]

mod state;
mod stm32f1xx;
mod usb;

extern crate panic_semihosting;

use crate::state::{ApplicationState, Button, Effect, Message};
use crate::stm32f1xx::read_input_pins;
use crate::stm32f1xx::{initialize_usb, Inputs};
use crate::usb::{configure_usb, usb_poll};
use stm32f1xx_hal::{
    gpio::gpioc::PC13,
    gpio::{Output, PushPull},
    pac::TIM1,
    prelude::*,
    timer::{CountDownTimer, Event, Timer},
    usb::{UsbBus, UsbBusType},
};
use usb_device::{
    bus,
    prelude::{UsbDevice, UsbDeviceState},
};
use usbd_midi::{
    data::usb_midi::usb_midi_event_packet::UsbMidiEventPacket,
    midi_device::MidiClass,
};

#[rtfm::app(device = stm32f1xx_hal::stm32,
            peripherals = true)]
const APP: () = {
    struct Resources {
        midi: MidiClass<'static, UsbBusType>,
        usb_dev: UsbDevice<'static, UsbBusType>,
        inputs: Inputs,
        led: PC13<Output<PushPull>>,
        timer: CountDownTimer<TIM1>,
        state: ApplicationState,
    }

    #[init()]
    fn init(cx: init::Context) -> init::LateResources {
        // This is a bit hacky, but gets us the static lifetime for the
        // allocator. Even when based on hardware initialization..
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        // Take ownership of IO devices
        let mut rcc = cx.device.RCC.constrain();
        let mut flash = cx.device.FLASH.constrain();
        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = cx.device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = cx.device.GPIOC.split(&mut rcc.apb2);
        let pa12 = gpioa.pa12;
        let pa11 = gpioa.pa11;
        let pb11 = gpiob.pb11.into_pull_up_input(&mut gpiob.crh);
        let pb12 = gpiob.pb12.into_pull_up_input(&mut gpiob.crh);
        let pb13 = gpiob.pb13.into_pull_up_input(&mut gpiob.crh);
        let pb14 = gpiob.pb14.into_pull_up_input(&mut gpiob.crh);
        let pb15 = gpiob.pb15.into_pull_up_input(&mut gpiob.crh);
        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
        let usb = cx.device.USB;

        // Configure clocks
        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        assert!(clocks.usbclk_valid());

        //Timer that will be used to read IO
        let mut timer = Timer::tim1(cx.device.TIM1, &clocks, &mut rcc.apb2)
            .start_count_down(100.hz());
        timer.listen(Event::Update);

        // Initialize usb resources
        // This is a bit tricky due to lifetimes in RTFM/USB playing
        // difficultly
        let usb = initialize_usb(&clocks, pa12, pa11, &mut gpioa.crh, usb);
        *USB_BUS = Some(UsbBus::new(usb));
        let midi = MidiClass::new(USB_BUS.as_ref().unwrap());
        let usb_dev = configure_usb(USB_BUS.as_ref().unwrap());

        let inputs = Inputs {
            pb11: pb11,
            pb12: pb12,
            pb13: pb13,
            pb14: pb14,
            pb15: pb15,
        };

        // Resources for RTFM
        init::LateResources {
            usb_dev: usb_dev,
            midi: midi,
            inputs: inputs,
            led: led,
            state: ApplicationState::init(),
            timer: timer,
        }
    }

    /// Will be called periodically.
    #[task(binds = TIM1_UP,
            spawn = [update],
            resources = [inputs,timer],
            priority = 1)]
    fn read_inputs(cx: read_inputs::Context) {
        // There must be a better way to bank over
        // these below checks

        let values = read_input_pins(cx.resources.inputs);

        let _ = cx.spawn.update((Button::One, values.pin1));
        let _ = cx.spawn.update((Button::Two, values.pin2));
        let _ = cx.spawn.update((Button::Three, values.pin3));
        let _ = cx.spawn.update((Button::Four, values.pin4));
        let _ = cx.spawn.update((Button::Five, values.pin5));

        cx.resources.timer.clear_update_interrupt_flag();
    }

    #[task( spawn = [send_midi],
            resources = [state],
            priority = 1,
            capacity = 5)]
    fn update(cx: update::Context, message: Message) {
        let effect = ApplicationState::update(*cx.resources.state, message);

        match effect {
            Effect::Midi(note) => {
                let _ = cx.spawn.send_midi(note);
            }
            Effect::Nothing => (),
        }
    }

    /// Sends a midi message over the usb bus
    /// Note: this runs at a lower priority than the usb bus
    /// and will eat messages if the bus is not configured yet
    #[task(priority=2, resources = [usb_dev,midi])]
    fn send_midi(cx: send_midi::Context, message: UsbMidiEventPacket) {
        let mut midi = cx.resources.midi;
        let mut usb_dev = cx.resources.usb_dev;

        // Lock this so USB interrupts don't take over
        // Ideally we may be able to better determine this, so that
        // it doesn't need to be locked
        usb_dev.lock(|usb_dev| {
            if usb_dev.state() == UsbDeviceState::Configured {
                midi.lock(|midi| {
                    let _ = midi.send_message(message);
                })
            }
        });
    }

    // Process usb events straight away from High priority interrupts
    #[task(binds = USB_HP_CAN_TX,resources = [usb_dev, midi], priority=3)]
    fn usb_hp_can_tx(mut cx: usb_hp_can_tx::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.midi);
    }

    // Process usb events straight away from Low priority interrupts
    #[task(binds= USB_LP_CAN_RX0, resources = [usb_dev, midi], priority=3)]
    fn usb_lp_can_rx0(mut cx: usb_lp_can_rx0::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.midi);
    }

    // Required for software tasks
    extern "C" {
        // Uses the DMA1_CHANNELX interrupts for software
        // task scheduling.
        fn DMA1_CHANNEL1();
        fn DMA1_CHANNEL2();
    }
};
