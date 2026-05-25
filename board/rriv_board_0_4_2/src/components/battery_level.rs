
use crate::*;
use embedded_hal::blocking::delay::DelayMs;

pub struct BatteryLevel {
  pins: pin_groups::BatteryLevelPins
}

impl BatteryLevel {
  pub fn new(pins: pin_groups::BatteryLevelPins) -> Self {
    return BatteryLevel {
      pins
    }
  }

  pub fn measure_battery_level(&mut self, adc: &mut InternalAdc, delay: &mut impl DelayMs<u16>) -> Result<f32, AdcError> {

    self.pins.enable_vin_measure.set_low();
    delay.delay_ms(1000_u16);
    let oversample_count = 10;
    let mut oversample_value: u32 = 0;
    for i in 0..oversample_count {
      match adc.read_battery_level(){
        Ok(value) => oversample_value = oversample_value + value as u32,
        Err(err) => return Err(err),
      }
    }
    self.pins.enable_vin_measure.set_high();
    let value = oversample_value / oversample_count;
    // convert to voltage
    let voltage = 3.3_f32 * value as f32 / 4096_f32;
    Ok( voltage )

  }
}

