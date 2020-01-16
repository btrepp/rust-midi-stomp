use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;
use stm32f1xx_hal::gpio::gpiob::{PB11, PB12, PB13, PB14, PB15};
use stm32f1xx_hal::gpio::Input;
use stm32f1xx_hal::gpio::PullUp;

/// Trait that represents a switch a user is pushing.
/// This functions with is_closed. Motivation is that
/// HALs would implemented this for pullup/pulldown inputs
pub trait Switch {
    type Error;
    fn is_closed(&self) -> Result<bool, Self::Error>;
}

//TODO macros..

impl Switch for PB11<Input<PullUp>> {
    type Error = Infallible;
    fn is_closed(&self) -> Result<bool, Self::Error> {
        self.is_low()
    }
}

impl Switch for PB12<Input<PullUp>> {
    type Error = Infallible;
    fn is_closed(&self) -> Result<bool, Self::Error> {
        self.is_low()
    }
}

impl Switch for PB13<Input<PullUp>> {
    type Error = Infallible;
    fn is_closed(&self) -> Result<bool, Self::Error> {
        self.is_low()
    }
}

impl Switch for PB14<Input<PullUp>> {
    type Error = Infallible;
    fn is_closed(&self) -> Result<bool, Self::Error> {
        self.is_low()
    }
}

impl Switch for PB15<Input<PullUp>> {
    type Error = Infallible;
    fn is_closed(&self) -> Result<bool, Self::Error> {
        self.is_low()
    }
}
