use crate::*;
use stm32f1xx_hal::adc;
use stm32f1xx_hal::pac::ADC1;

pub struct InternalAdcConfiguration {
  pins: pin_groups::InternalAdcPins,
  adc_device: ADC1,
}

pub struct InternalAdc {
  pins: pin_groups::InternalAdcPins,
  adc: adc::Adc<ADC1>,
}

pub enum AdcError {
  NBError(nb::Error<()>),
  NotConfigured,
  ReadError,
}

impl InternalAdcConfiguration {
  pub fn new(pins: pin_groups::InternalAdcPins, adc1: ADC1) -> Self {
    return InternalAdcConfiguration {
      pins,
      adc_device: adc1,
    }
  }

  pub fn build(self, clocks: &Clocks) -> InternalAdc {

    let adc_device = self.adc_device;
    let adc = adc::Adc::adc1(adc_device, *clocks);
    return InternalAdc::new(self.pins, adc);
  }
}

impl InternalAdc {
  pub fn new(pins: pin_groups::InternalAdcPins, adc: adc::Adc<ADC1>) -> Self {

    return InternalAdc {
      pins,
      adc,
    }
  }

  pub fn enable(&mut self, delay: &mut SysDelay) {
    self.pins.enable_avdd.set_low();
    delay.delay_ms(100_u16);
  }

  pub fn disable(&mut self){
    self.pins.enable_avdd.set_high();
  }


  pub fn shutdown(mut self) -> InternalAdcConfiguration {
    self.disable();
    let adc_device = self.adc.release();
    return InternalAdcConfiguration::new(self.pins, adc_device);

  }

  pub fn read(&mut self, channel: u8) -> Result<u16, AdcError> {

      let res: Result<u16, nb::Error<()>> = match channel {
        1 => self.adc.read(&mut self.pins.channel1),
        2 => self.adc.read(&mut self.pins.channel2),
        3 => self.adc.read(&mut self.pins.channel3),
        4 => self.adc.read(&mut self.pins.channel4),
        5 => self.adc.read(&mut self.pins.channel5),
        _ => return Err(AdcError::ReadError)
      };

     
      match res {
        Ok(value) => return Ok(value),
        Err(error) =>  return Err(AdcError::NBError(error)), 
      }
   
  }

  pub fn read_battery_level(&mut self) -> Result<u16, AdcError>  {
    
    let res = self.adc.read(&mut self.pins.vin_measure);
    
    match res {
      Ok(value) => return Ok(value),
      Err(error) =>  return Err(AdcError::NBError(error)), 
    }
    
  }


}