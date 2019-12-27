#![no_std]
#![no_main]

// pick a panicking behavior
extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// extern crate panic_abort; // requires nightly
// extern crate panic_itm; // logs messages over ITM; requires ITM support
// extern crate panic_semihosting; // logs messages to the host stderr; requires a debugger

use cortex_m::asm::delay;
use embedded_hal::digital::v2::OutputPin;
use stm32f1xx_hal::{
    prelude::*,
    usb::{Peripheral, UsbBus, UsbBusType},
};
use rtfm::cyccnt::U32Ext;
use usb_device::prelude::UsbDevice;
use usb_device::bus;
use usb_device::prelude::UsbDeviceState;
use usb_device::prelude::UsbDeviceBuilder;
use usb_device::prelude::UsbVidPid;
use usbd_midi::midi_device::MidiClass;
use usbd_midi::data::usb::constants::USB_CLASS_NONE;

use usbd_midi:: {
    data:: {
        byte::u7::U7,
        midi::notes::Note,
        midi::message::Message,
        midi::channel::Channel,
        usb_midi::cable_number::CableNumber,
        usb_midi::usb_midi_event_packet::UsbMidiEventPacket
    }
};

const CABLE :CableNumber = CableNumber::Cable0;
const CHANNEL :Channel= Channel::Channel1;
const NOTE: Note = Note::C3;
const VELOCITY: U7 = U7::MAX;

const NOTE_ON : UsbMidiEventPacket = {
    const MIDI :Message= Message::NoteOn(CHANNEL,NOTE,VELOCITY);
    UsbMidiEventPacket{
        cable_number : CABLE,
        message : MIDI
    }
};

const NOTE_OFF : UsbMidiEventPacket = {
    const MIDI :Message= Message::NoteOff(CHANNEL,NOTE,VELOCITY);
    UsbMidiEventPacket{
        cable_number : CABLE,
        message : MIDI
    }
};

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

#[rtfm::app(device = stm32f1xx_hal::stm32,peripherals = true,monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        midi: MidiClass<'static,UsbBusType>,
        usb_dev: UsbDevice<'static,UsbBusType>
    }

    #[init(spawn= [send_midi])]
    fn init(mut cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();
        let device = cx.device;

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();
        
        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

        assert!(clocks.usbclk_valid());

        // BluePill board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        usb_dp.set_low().unwrap();
        delay(clocks.sysclk().0 / 100);

        let usb_dm = gpioa.pa11;
        let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);

        let usb = Peripheral {
            usb: device.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp
        };

        *USB_BUS = Some(UsbBus::new(usb));

        let midi = MidiClass::new(USB_BUS.as_ref().unwrap());
        
        let usb_dev =
            UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Unknown")
                .product("BluepillX Midi")
                .serial_number("TOTALLY_LEGIT")
                .device_class(USB_CLASS_NONE)
                .build();

        cx.spawn.send_midi().unwrap();                

        init::LateResources {
            usb_dev : usb_dev,
            midi : midi
        }
    }


    #[task(schedule = [send_midi], priority=1, resources = [usb_dev,midi])]
    fn send_midi(cx: send_midi::Context){

        static mut ON:bool = false;

        if cx.resources.usb_dev.state() == UsbDeviceState::Configured {

            if *ON {
                let _ = cx.resources.midi.send_message(NOTE_ON);
                
            } else {
                let _ = cx.resources.midi.send_message(NOTE_OFF);
                
            }

            *ON = !*ON;
            
        }
        cx.schedule.send_midi(cx.scheduled+32_000_000.cycles()).unwrap();
    }

    // Process usb events straight away from High priority interrupts
    #[task(binds = USB_HP_CAN_TX,resources = [usb_dev, midi])]
    fn usb_hp_can_tx(mut cx: usb_hp_can_tx::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.midi);
    }

    // Process usb events straight away from Low priority interrupts
    #[task(binds= USB_LP_CAN_RX0, resources = [usb_dev, midi])]
    fn usb_lp_can_rx0(mut cx:usb_lp_can_rx0::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.midi);
    }

    // Required for software tasks
    extern "C" {

        // Uses the DMA1_CHANNEL1 interrupts for software
        // task scheduling.
        fn DMA1_CHANNEL1();
    }
};

