#![no_std]
#![no_main]

mod constants;

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
    gpio:: {gpioa::PA4, Input,PullDown,Floating, ExtiPin, Edge}
};
use rtfm::cyccnt::U32Ext;
use usb_device::prelude::UsbDevice;
use usb_device::bus;
use usb_device::prelude::UsbDeviceState;
use usb_device::prelude::UsbDeviceBuilder;
use usb_device::prelude::UsbVidPid;
use usbd_midi::midi_device::MidiClass;
use usbd_midi::data::usb::constants::USB_CLASS_NONE;
use crate::constants::{NOTE_OFF,NOTE_ON};
use usbd_midi::data::usb_midi::usb_midi_event_packet::UsbMidiEventPacket;

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


#[rtfm::app(device = stm32f1xx_hal::stm32,
            peripherals = true,
            monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        midi: MidiClass<'static,UsbBusType>,
        usb_dev: UsbDevice<'static,UsbBusType>,
        pa4 : PA4<Input<PullDown>>,
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
        let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);
        let pa12 = gpioa.pa12;
        let pa11 = gpioa.pa11;
        let mut pa4 = gpioa.pa4.into_pull_down_input(&mut gpioa.crl);
        let usb = cx.device.USB;

        // Configure clocks
        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);       

        assert!(clocks.usbclk_valid());

        // Configure digital interrupts
        pa4.make_interrupt_source(&mut afio);
        pa4.trigger_on_edge(&cx.device.EXTI, Edge::RISING_FALLING);
        pa4.enable_interrupt(&cx.device.EXTI);


        // Initialize usb resources
        // This is a bit tricky due to lifetimes in RTFM/USB playing
        // difficultly
        let usb = initialize_usb(&clocks,pa12,pa11,&mut gpioa.crh,usb);
        *USB_BUS = Some(UsbBus::new(usb));
        let midi = MidiClass::new(USB_BUS.as_ref().unwrap());
        let usb_dev = configure_usb(USB_BUS.as_ref().unwrap());

        // Start main reading of IO
        // Will be uncessary if we can use interrupts instead
        //cx.spawn.main_loop().unwrap();            

        // Resources for RTFM
        init::LateResources {
            usb_dev : usb_dev,
            midi : midi,
            pa4: pa4
        }
    }


    #[task(binds = EXTI4, spawn = [send_midi], resources = [pa4], priority = 1)]
    fn read_inputs(cx:read_inputs::Context) {
        let message = NOTE_ON;
        let _ = cx.spawn.send_midi(message);
        cx.resources.pa4.clear_interrupt_pending_bit();
    }

    /// Main 'loop'
    /// currently this runs every 1_000_000 cycles,
    /// reads the input pin, and converts sends messages
    /// on an 'edge'
    #[task( schedule = [main_loop],
            spawn = [send_midi], 
            priority=1,
            resources = [pa4])]
    fn main_loop(cx:main_loop::Context){
        static mut LAST_RESULT:bool = false;
        let pin = cx.resources.pa4;
        let result = pin.is_high().unwrap();

        // Only send midi if value changes
        if result != *LAST_RESULT {

            let message = if result { NOTE_ON} else {NOTE_OFF};
            // if sucessfully sent, update last result
            // if not we will try again next time
            let queued = cx.spawn.send_midi(message);
            match queued {
                Ok(_) => {*LAST_RESULT = result;},
                _ => ()
            }
        }
        // Run this function again in future.
        // Result type is ignored because if it's already scheduled thats okay
        let _ = cx.schedule.main_loop(cx.scheduled+1_000_000.cycles());
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

