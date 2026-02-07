use rtt_target::rprint;
use serde_json::json;
use util::any_as_u8_slice;

use crate::sensor_name_from_type_id;

use super::types::*;

#[derive(Copy, Clone)]
pub struct ADCTemperatureDriverSpecialConfiguration {
    empty: [u8; 32], // must add to 32
}

impl ADCTemperatureDriverSpecialConfiguration {
    pub fn parse_from_values(
        value: serde_json::Value,
    ) -> Result<ADCTemperatureDriverSpecialConfiguration, &'static str>  {
        Ok(Self {
            empty: [b'\0'; 32] 
        })
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> ADCTemperatureDriverSpecialConfiguration {
        let settings = bytes.as_ptr().cast::<ADCTemperatureDriverSpecialConfiguration>();
        unsafe { *settings } 
    }
 
}

const NUMBER_OF_MEASURED_PARAMETERS : usize = 1;

pub struct ADCTemperatureDriver {
    general_config: SensorDriverGeneralConfiguration,
    special_config: ADCTemperatureDriverSpecialConfiguration,
    measured_parameter_values: [f64; NUMBER_OF_MEASURED_PARAMETERS],
}

impl SensorDriver for ADCTemperatureDriver {

    fn get_configuration_json(&mut self) -> serde_json::Value  {

        let sensor_name_bytes = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name_str = core::str::from_utf8(&sensor_name_bytes).unwrap_or_default();

        json!({ 
            "id" : self.get_id(),
            "type" : sensor_name_str,
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
    }

    getters!();

    fn get_measured_parameter_count(&mut self) -> usize {
        NUMBER_OF_MEASURED_PARAMETERS
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        if self.measured_parameter_values[index] == f64::MAX {
            Err(())
        } else {
            Ok(self.measured_parameter_values[index])
        }
    }

    #[allow(unused)]
    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8;16] {
        return [b't', b'e', b'm', b'p', b'e', b'r', b'a', b't', b'u', b'r', b'e', b'\0', b'\0', b'\0', b'\0', b'\0'];
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        
        self.measured_parameter_values[0] = board.read_temp_adc() as f64; // example conversion
    }

    fn clear_calibration(&mut self) {

    }

    #[allow(unused)]
    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
       // validation
        Ok(())
    }
   
    #[allow(unused)]
    fn update_actuators(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
    }
        
}

impl ADCTemperatureDriver {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: ADCTemperatureDriverSpecialConfiguration,
    ) -> Self {
        ADCTemperatureDriver {
            general_config,
            special_config,
            measured_parameter_values: [0.0; NUMBER_OF_MEASURED_PARAMETERS],
        }
    }

}