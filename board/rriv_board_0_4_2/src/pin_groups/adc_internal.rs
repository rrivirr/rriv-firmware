use stm32f1xx_hal::{gpio::*, afio::MAPR};
use crate::pins::*;


pub struct InternalAdcPins {
    pub enable_avdd: Pin<'A', 1, Output>,
    pub channel1: Pin<'A', 0, Analog>,
    pub channel2: Pin<'C', 3, Analog>,
    pub channel3: Pin<'C', 2, Analog>,
    pub channel4: Pin<'C', 1, Analog>,
    pub channel5: Option<Pin<'C', 0, Analog>>,
    pub vin_measure: Pin<'B', 0, Analog>,

}

impl InternalAdcPins {
    pub fn build(
        enable_avdd: Pin<'A', 1>,
        channel1: Pin<'A', 0>,
        channel2: Pin<'C', 3>,
        channel3: Pin<'C', 2>,
        channel4: Pin<'C', 1>,
        channel5: Pin<'C', 0>,
        vin_measure: Pin<'B', 0>,
        cr: &mut GpioCr,
    ) -> Self {
        return InternalAdcPins {
            enable_avdd: enable_avdd.into_push_pull_output(&mut cr.gpioa_crl),
            channel1: channel1.into_analog(&mut cr.gpioa_crl),
            channel2: channel2.into_analog(&mut cr.gpioc_crl),
            channel3: channel3.into_analog(&mut cr.gpioc_crl),
            channel4: channel4.into_analog(&mut cr.gpioc_crl),
            channel5: Some(channel5.into_analog(&mut cr.gpioc_crl)),
            vin_measure: vin_measure.into_analog(&mut cr.gpiob_crl)
        };
    }

    pub fn take_port_5(&mut self) -> Pin<'C', 0, stm32f1xx_hal::gpio::Analog> {
        let port5 = self.channel5.take();
        let port5 = port5.unwrap();
        return port5;
    }
}