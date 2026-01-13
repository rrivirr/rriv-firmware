use stm32f1xx_hal::gpio::*;
use crate::pins::*;

// The 2nd SPI interface, for future use
pub struct Spi1Pins {
  #[allow(unused)]
  pub sck: Pin<'A', 5, Alternate>,
  #[allow(unused)]
  pub miso: Pin<'A', 6>,
  #[allow(unused)]
  pub mosi: Pin<'A', 7, Alternate>,
  // pub sd_card_chip_select: Pin<'C', 8, Output>
}

impl Spi1Pins {
  pub fn build(
    sck: Pin<'A', 5>,
    miso: Pin<'A', 6>,
    mosi: Pin<'A', 7>,
    // sd_card_chip_select: Pin<'C', 8>,
    cr: &mut GpioCr
  ) -> Self {

    Spi1Pins {
      sck: sck.into_alternate_push_pull(&mut cr.gpioa_crl),
      miso,
      mosi: mosi.into_alternate_push_pull(&mut cr.gpioa_crl),
      // sd_card_chip_select: sd_card_chip_select.into_push_pull_output(&mut cr.gpioc_crh),
    }
  }

}