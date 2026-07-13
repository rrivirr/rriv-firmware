use crate::sensor_name_from_type_id;

use super::types::*;
use serde_json::json;

pub struct BatteryLevel {
    general_config: SensorDriverGeneralConfiguration,
    special_config: EmptySpecialConfiguration,
    measured_parameter_values: [f64; 1],
}

impl SensorDriver for BatteryLevel {
    fn get_configuration_json(&mut self) -> serde_json::Value {
        
        let mut sensor_id = self.get_id();
        let sensor_id = match util::str_from_utf8(&mut sensor_id) {
            Ok(sensor_id) => sensor_id,
            Err(_) => "Invalid",
        };

        let mut sensor_name = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name = match util::str_from_utf8(&mut sensor_name) {
            Ok(sensor_name) => sensor_name,
            Err(_) => "Invalid",
        };

        json!({
           "id" : sensor_id,
           "type" : sensor_name
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
    }
 
    getters!();

    fn get_measured_parameter_count(&mut self) -> usize {
        return 1;
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        return Ok(self.measured_parameter_values[index]);
    }

    fn get_measured_parameter_identifier(&mut self, _index: usize) -> [u8; 16] {

        let identifier: &str = "V";
        let mut buf: [u8; 16] = [0_u8; 16];     
        for i in 0..identifier.len() {
            buf[i] = identifier.as_bytes()[i];
        }
        return buf;
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        defmt::println!("get the battery level");
        let value = board.get_battery_level();
        defmt::println!("got the battery level");
        self.measured_parameter_values[0] = value.into();
        defmt::println!("stored the battery level");
    }



}

impl BatteryLevel {
    // pub fn new(general_config: SensorDriverGeneralConfiguration, special_config_bytes: &[u8; rriv_board::EEPROM_SENSOR_SPECIAL_SETTINGS_SIZE]) -> Self {

    //     let special_config = GenericAnalogSpecialConfiguration::new_from_bytes(special_config_bytes);
    //     GenericAnalog {
    //         general_config,
    //         special_config
    //     }
    // }

    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: EmptySpecialConfiguration,
    ) -> Self {
        BatteryLevel {
            general_config,
            special_config,
            measured_parameter_values: [0.0; 1],
        }
    }
}
