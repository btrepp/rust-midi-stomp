#![no_std]
#![no_main]

mod state;

// pick a panicking behavior
extern crate panic_semihosting; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// extern crate panic_abort; // requires nightly
// extern crate panic_itm; // logs messages over ITM; requires ITM support
// extern crate panic_semihosting; // logs messages to the host stderr; requires a debugger


use cortex_m::asm::delay;
use embedded_hal::digital::v2::{
    OutputPin,
    InputPin
};
use stm32f1xx_hal::{
    prelude::*,
    usb::{Peripheral, UsbBus, UsbBusType},
    gpio::{
        gpiob::{PB11,PB12,PB13,PB14,PB15},
        gpioc::PC13
    },
    gpio:: {PushPull, Output, Input,PullUp,Floating},
    timer::{Timer,CountDownTimer,Event},
    pac::{TIM1}
};
use rtfm::cyccnt::U32Ext;
use usb_device::{
    prelude::{
        UsbDevice,
        UsbDeviceState,
        UsbDeviceBuilder,
        UsbVidPid
    },
    bus
};
use usbd_midi::{
    midi_device::MidiClass,
    data::usb::constants::USB_CLASS_NONE,
    data::usb_midi::usb_midi_event_packet::UsbMidiEventPacket
};
use crate::state::{ApplicationState,Button,State,Message,Effect};

/// Called to process any usb events
/// Note: this needs to be called often,
/// and seemingly always the same way
fn usb_poll<B: bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    midi: &mut MidiClass<'static, B>,
) {
    if !usb_dev.poll(&mut [midi]) {
        return;
    }   
}

/// Initializes the bluepill usb stack.
/// This will also set the dp line low. To RESET
/// the usb bus
fn initialize_usb(
                clocks:&stm32f1xx_hal::rcc::Clocks,
                pa12:stm32f1xx_hal::gpio::gpioa::PA12<Input<Floating>>,
                pa11:stm32f1xx_hal::gpio::gpioa::PA11<Input<Floating>>,
                crh: &mut stm32f1xx_hal::gpio::gpioa::CRH,
                usb:stm32f1xx_hal::stm32::USB) 
                -> stm32f1xx_hal::usb::Peripheral {
                            
    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    let mut usb_dp = pa12.into_push_pull_output(crh);
    usb_dp.set_low().unwrap();
    delay(clocks.sysclk().0 / 100);

    let usb_dm = pa11;
    let usb_dp = usb_dp.into_floating_input(crh);

    let usb = Peripheral {
        usb: usb,
        pin_dm: usb_dm,
        pin_dp: usb_dp
    };

    usb
}

/// Configures the usb device as seen by the operating system.
fn configure_usb<'a>(usb_bus: &'a bus::UsbBusAllocator<UsbBusType>) 
                                        -> UsbDevice<'a,UsbBusType> {
    let usb_vid_pid = UsbVidPid(0x16c0, 0x27dd);
    let usb_dev =
        UsbDeviceBuilder::new(usb_bus,usb_vid_pid )
            .manufacturer("btrepp")
            .product("Rust Midi Stomp")
            .serial_number("1")
            .device_class(USB_CLASS_NONE)
            .build();   
    usb_dev
}

pub struct Inputs {
    pb11 : PB11<Input<PullUp>>,
    pb12 : PB12<Input<PullUp>>,
    pb13 : PB13<Input<PullUp>>,
    pb14 : PB14<Input<PullUp>>,
    pb15 : PB15<Input<PullUp>>
}

#[rtfm::app(device = stm32f1xx_hal::stm32,
            peripherals = true,
            monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        midi: MidiClass<'static,UsbBusType>,
        usb_dev: UsbDevice<'static,UsbBusType>,
        inputs: Inputs,
        led : PC13<Output<PushPull>>,
        timer: CountDownTimer<TIM1>,
        state : ApplicationState
    }

    #[init(spawn= [main_loop])]
    fn init(mut cx: init::Context) -> init::LateResources {
        // This is a bit hacky, but gets us the static lifetime for the 
        // allocator. Even when based on hardware initialization..
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        // Enables timers so scheduling works
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

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
        let usb = initialize_usb(&clocks,pa12,pa11,&mut gpioa.crh,usb);
        *USB_BUS = Some(UsbBus::new(usb));
        let midi = MidiClass::new(USB_BUS.as_ref().unwrap());
        let usb_dev = configure_usb(USB_BUS.as_ref().unwrap());

        // Start the monitoring loop
        cx.spawn.main_loop().unwrap();            

        let inputs = 
            Inputs {
                pb11: pb11,
                pb12: pb12,
                pb13: pb13,
                pb14: pb14,
                pb15: pb15,
            };
        // Resources for RTFM
        init::LateResources {
            usb_dev : usb_dev,
            midi : midi,
            inputs: inputs,
            led: led,
            state : ApplicationState::init(),
            timer: timer
        }
    }

    /// Will be called periodically.
    #[task(binds = TIM1_UP, 
            spawn = [update],
            resources = [inputs,timer], 
            priority = 1)]
    fn read_inputs(cx:read_inputs::Context) {
        // There must be a better way to bank over
        // these below checks

        let inputs = cx.resources.inputs;

        let pb11 = match inputs.pb11.is_high(){
                        Ok(true) => (Button::One,State::On),
                        _ => (Button::One,State::Off)
                    };
        let pb12 = match inputs.pb12.is_high(){
            Ok(true) => (Button::Two,State::On),
            _ => (Button::Two,State::Off)
        };
        let pb13 = match inputs.pb13.is_high(){
            Ok(true) => (Button::Three,State::On),
            _ => (Button::Three,State::Off)
        };
        let pb14 = match inputs.pb14.is_high(){
            Ok(true) => (Button::Four,State::On),
            _ => (Button::Four,State::Off)
        };

        let pb15 = match inputs.pb15.is_high(){
            Ok(true) => (Button::Five,State::On),
            _ => (Button::Five,State::Off)
        };

        let _ = cx.spawn.update(pb11);
        let _ = cx.spawn.update(pb12);
        let _ = cx.spawn.update(pb13);
        let _ = cx.spawn.update(pb14);
        let _ = cx.spawn.update(pb15);

        cx.resources.timer.clear_update_interrupt_flag();
    }

    #[task( spawn = [send_midi],
            resources = [state],
            priority = 1,
            capacity = 5)]
    fn update(cx:update::Context, message:Message) {
        let (state,effect) = 
                    ApplicationState::update(*cx.resources.state,message);

        match effect {
            Effect::Midi(note) => {
                let _ = cx.spawn.send_midi(note);
            },
            Effect::Nothing => ()
        }
        *cx.resources.state = state;
    }

    /// Main 'loop'
    /// Currently used to toggle the bluepill led to show the device is
    /// functioning correctly
    #[task( schedule = [main_loop],
            priority=1,
            resources = [led])]
    fn main_loop(cx:main_loop::Context){
        let _ = cx.resources.led.toggle();

        // Run this function again in future.
        // Result type is ignored because if it's already scheduled thats okay
        let _ = cx.schedule.main_loop(cx.scheduled+32_000_000.cycles());
    }

    /// Sends a midi message over the usb bus
    /// Note: this runs at a lower priority than the usb bus
    /// and will eat messages if the bus is not configured yet
    #[task(priority=2, resources = [usb_dev,midi])]
    fn send_midi(cx: send_midi::Context, message:UsbMidiEventPacket){
        let mut midi = cx.resources.midi;
        let mut usb_dev = cx.resources.usb_dev;

        // Lock this so USB interrupts don't take over
        // Ideally we may be able to better determine this, so that
        // it doesn't need to be locked
        usb_dev.lock(|usb_dev|{
            if usb_dev.state() == UsbDeviceState::Configured{
                midi.lock(|midi|{
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
    fn usb_lp_can_rx0(mut cx:usb_lp_can_rx0::Context) {
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

