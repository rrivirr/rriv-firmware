use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32f1xx_hal::gpio::{Dynamic, Pin, PinModeError};


pub struct OneWirePin<P> {
    pub pin: P
}

impl InputPin for OneWirePin<Pin<'D', 2, Dynamic>> {
    type Error = PinModeError;

    fn is_high(&self) -> Result<bool, Self::Error> {
        unsafe { Ok((*crate::pac::GPIOD::ptr()).idr.read().bits() & (1 << 2) != 0) }
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        // unsafely access the pin state of GPIO B8
        // because the hal doesn't currently implement a IO pin type later change to that
        // this is safe because this is a one wire protocol
        // and we don't need the mode of the pin to be checked.
        unsafe { Ok((*crate::pac::GPIOD::ptr()).idr.read().bits() & (1 << 2) == 0) }
    }
}

impl OutputPin for OneWirePin<Pin<'D', 2, Dynamic>> {
    type Error = PinModeError;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { Ok((*crate::pac::GPIOD::ptr()).odr.write(|w| w.odr2().clear_bit()))}
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { Ok((*crate::pac::GPIOD::ptr()).odr.write(|w| w.odr2().set_bit()))}
    }
}


impl InputPin for OneWirePin<Pin<'C',0, Dynamic>> {
    type Error = PinModeError;

    fn is_high(&self) -> Result<bool, Self::Error> {
        unsafe { Ok((*crate::pac::GPIOC::ptr()).idr.read().bits() & (1) != 0) }
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        unsafe { Ok((*crate::pac::GPIOC::ptr()).idr.read().bits() & (1) == 0) }
    }
}

impl OutputPin for OneWirePin<Pin<'C', 0, Dynamic>> {
    type Error = PinModeError;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { Ok((*crate::pac::GPIOC::ptr()).odr.write(|w| w.odr0().clear_bit()))}
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { Ok((*crate::pac::GPIOC::ptr()).odr.write(|w| w.odr0().set_bit()))}
    }
}