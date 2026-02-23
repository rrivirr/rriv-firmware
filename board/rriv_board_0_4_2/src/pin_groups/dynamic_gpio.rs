use stm32f1xx_hal::gpio::*;
use crate::pins::*;

pub struct DynamicGpioPins {
    pub gpio1: Pin<'B', 8, Alternate<OpenDrain>>,
    pub gpio2: Pin<'B', 5, Dynamic>,
    pub gpio3: Pin<'B', 4, Dynamic>,
    pub gpio4: Pin<'B', 3, Dynamic>,
    pub gpio5: Pin<'D', 2, Dynamic>,
    pub gpio6: Pin<'C', 12, Dynamic>,
    pub gpio7: Pin<'C', 11, Dynamic>,
    pub gpio8: Pin<'C', 10, Dynamic>,
}

impl DynamicGpioPins {
    pub fn build(
        gpio1: Pin<'B', 8>,
        gpio2: Pin<'B', 5>,
        gpio3: Pin<'B', 4>,
        gpio4: Pin<'B', 3>,
        gpio5: Pin<'D', 2>,
        gpio6: Pin<'C', 12>,
        gpio7: Pin<'C', 11>,
        gpio8: Pin<'C', 10>,
        cr: &mut GpioCr,
    ) -> Self {
        return DynamicGpioPins {
            gpio1: gpio1.into_alternate_open_drain(&mut cr.gpiob_crh),
            gpio2: gpio2.into_dynamic(&mut cr.gpiob_crl),
            gpio3: gpio3.into_dynamic(&mut cr.gpiob_crl),
            gpio4: gpio4.into_dynamic(&mut cr.gpiob_crl),
            gpio5: gpio5.into_dynamic(&mut cr.gpiod_crl),
            gpio6: gpio6.into_dynamic(&mut cr.gpioc_crh),
            gpio7: gpio7.into_dynamic(&mut cr.gpioc_crh),
            gpio8: gpio8.into_dynamic(&mut cr.gpioc_crh),
        };
    }
}