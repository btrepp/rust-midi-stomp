
use crate::state::State;
use cortex_m::asm::delay;
use stm32f1xx_hal::usb::Peripheral;
use stm32f1xx_hal::gpio::gpiob::{PB11,PB12,PB13,PB14,PB15};
use stm32f1xx_hal::gpio::PullUp;
use stm32f1xx_hal::gpio::Floating;
use stm32f1xx_hal::gpio::Input;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::v2::InputPin;

pub struct Inputs {
    pub pb11 : PB11<Input<PullUp>>,
    pub pb12 : PB12<Input<PullUp>>,
    pub pb13 : PB13<Input<PullUp>>,
    pub pb14 : PB14<Input<PullUp>>,
    pub pb15 : PB15<Input<PullUp>>
}

pub struct InputRead {
    pub pin1: State,
    pub pin2: State,
    pub pin3: State,
    pub pin4: State,
    pub pin5: State
}

/// Initializes the bluepill usb stack.
/// This will also set the dp line low. To RESET
/// the usb bus
pub fn initialize_usb(
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

pub fn read_input_pins(inputs:&mut Inputs) -> InputRead {
    InputRead {
        pin1 : inputs.pb11.is_high().unwrap_or(false).into(),
        pin2 : inputs.pb12.is_high().unwrap_or(false).into(),
        pin3 : inputs.pb12.is_high().unwrap_or(false).into(),
        pin4 : inputs.pb12.is_high().unwrap_or(false).into(),
        pin5 : inputs.pb12.is_high().unwrap_or(false).into(),
    }
}