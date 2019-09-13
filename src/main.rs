#![no_std]
#![no_main]

// pick a panicking behavior
extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// extern crate panic_abort; // requires nightly
// extern crate panic_itm; // logs messages over ITM; requires ITM support
// extern crate panic_semihosting; // logs messages to the host stderr; requires a debugger

use cortex_m::asm::delay;
use embedded_hal::digital::v2::OutputPin;
use rtfm::app;
use stm32f1xx_hal::{
    prelude::*
};
use usb_device::prelude::UsbDevice;
use usb_device::bus;
use usb_device::prelude::UsbDeviceBuilder;
use usb_device::prelude::UsbVidPid;
use stm32_usbd::{UsbBus,UsbBusType};
mod midi;

#[app(device = stm32f1::stm32f103)]
const APP: () = {

    static mut USB_DEV : UsbDevice<'static, UsbBusType> = ();
    static mut MIDI : midi::MidiClass<'static, UsbBusType> = ();

    #[init()]
    fn init() -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        let _core : rtfm::Peripherals = core;
        let device : stm32f1::stm32f103::Peripherals = device;

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


        *USB_BUS = Some(UsbBus::new(device.USB, (usb_dm,usb_dp)));

        let midi = midi::MidiClass::new(USB_BUS.as_ref().unwrap());
        
        let usb_dev =
            UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Unknown")
                .product("BluepillX Midi")
                .serial_number("TOTALLY_LEGIT")
                .device_class(midi::USB_CLASS_NONE)
                .build();
        init::LateResources {
            USB_DEV : usb_dev,
            MIDI : midi
        }
    }


    #[interrupt(resources = [USB_DEV, MIDI])]
    fn USB_HP_CAN_TX() {
        usb_poll(&mut resources.USB_DEV, &mut resources.MIDI);
    }

    #[interrupt(resources = [USB_DEV, MIDI])]
    fn USB_LP_CAN_RX0() {
        usb_poll(&mut resources.USB_DEV, &mut resources.MIDI);
    }

    extern "C" {
        fn DMA1_CHANNEL1();
    }
};

fn usb_poll<B: bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    midi: &mut midi::MidiClass<'static, B>,
) {
    if !usb_dev.poll(&mut [midi]) {
        return;
    }
   
}