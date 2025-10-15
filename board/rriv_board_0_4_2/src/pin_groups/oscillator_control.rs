use stm32f1xx_hal::gpio::*;
use crate::pins::*;

pub struct OscillatorControlPins {
  pub enable_hse: Pin<'C', 13, Output>,
}

impl OscillatorControlPins {
  pub fn build(enable_hse: Pin<'C', 13>, cr: &mut GpioCr) -> Self {
      Self {
          enable_hse: enable_hse.into_push_pull_output(&mut cr.gpioc_crh),
      }
  }
}