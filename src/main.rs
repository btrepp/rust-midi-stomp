#![no_std]
#![no_main]

// pick a panicking behavior
extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// extern crate panic_abort; // requires nightly
// extern crate panic_itm; // logs messages over ITM; requires ITM support
// extern crate panic_semihosting; // logs messages to the host stderr; requires a debugger

use embedded_hal::digital::v2::OutputPin;
use cortex_m_semihosting::{hprintln};
use rtfm::app;
use stm32f1xx_hal::{
    prelude::*,
    gpio:: {
        PushPull,
        Output,
        gpioc
    }
};


#[app(device = stm32f1::stm32f103)]
const APP: () = {

    static mut PIN : gpioc::PC13<Output<PushPull>> = ();
    #[init(spawn= [blink])]
    fn init() -> init::LateResources {
        let _core : rtfm::Peripherals = core;
        let device : stm32f1::stm32f103::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);
        
        let pc13 = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        spawn.blink().unwrap();

        init::LateResources {
            PIN : pc13
        }
    }

    #[task(schedule = [blink],resources = [PIN])]
    fn blink() {
          //Its own state
        static mut STATE: bool = false;
        let led = resources.PIN;
        if *STATE ==true {
            let _ = led.set_low();
        } else {
            let _ = led.set_high();
        }
        hprintln!("{}", *STATE).unwrap();
        //Flip state
        *STATE = !*STATE;
        //Schedule itself to run again
        let next = scheduled+8_000_000.cycles();
        schedule.blink(next).unwrap();

    }

    extern "C" {
        fn DMA1_CHANNEL1();
    }
};
