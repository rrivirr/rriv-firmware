use crate::sensor_name_from_type_id;

use super::types::*;
use bitfield_struct::bitfield;
use serde_json::json;

pub struct GenericAnalog {
    general_config: SensorDriverGeneralConfiguration,
    special_config: GenericAnalogSpecialConfiguration,
    measured_parameter_values: [f64; 2],
    m: f64,
    b: f64,
}

impl SensorDriver for GenericAnalog {
    fn get_configuration_json(&mut self) -> serde_json::Value {
        let adc_select = match self.special_config.settings.adc_select() {
            0 => "internal",
            1 => "external",
            _ => "invalid",
        };

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
           "type" : sensor_name,
           "m": self.m, // return the converted value
           "b" : self.b, // return the converted value
           "sensor_port": self.special_config.sensor_port,
           "adc_select": adc_select
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        self.m = self.special_config.m as f64;
        // defmt::println!("loading {:#b} {} {} {}", self.special_config.b, self.special_config.b, self.special_config.b as f64, (self.special_config.b as f64) / 1000_f64 );
        self.b = self.special_config.b as f64;
    }
 
    getters!();

    fn get_measured_parameter_count(&mut self) -> usize {
        return 2;
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        return Ok(self.measured_parameter_values[index]);
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        return single_raw_or_cal_parameter_identifiers(index, None);
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        let value = 
            match self.special_config.settings.adc_select() {
                0 => {
                    board.query_internal_adc(self.special_config.sensor_port)
                }
                1 => {
                    board.query_external_adc(self.special_config.sensor_port)
                }
                2_usize.. => todo!("other adcs not implemented"),
            };
        self.measured_parameter_values[0] = value.into();
        self.measured_parameter_values[1] = self.m * value as f64 + self.b;
    }

    fn clear_calibration(&mut self) {
        self.m = 0_f64;
        self.b = 0_f64;
        self.special_config.m = 0_f32;
        self.special_config.b = 0_f32;
    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
        for i in 0..pairs.len() {
            let pair = &pairs[i];
            defmt::println!("calib pair{:?} {} {}", i, pair.point, pair.values[0]);
        }

        if pairs.len() != 2 {
            return Err(());
        }

        let cal1 = &pairs[0];
        let cal2 = &pairs[1];

        self.m = (cal2.point - cal1.point) / (cal2.values[0] - cal1.values[0]);
        self.b = cal1.point - self.m * cal1.values[0];
        defmt::println!("calibration: {} {}", self.m, self.b);
        self.special_config.m = self.m as f32;
        self.special_config.b = self.b as f32;
        defmt::println!(
            "calibration: {} {}",
            self.special_config.m,
            self.special_config.b
        );

        Ok(())
    }

}

impl GenericAnalog {
    // pub fn new(general_config: SensorDriverGeneralConfiguration, special_config_bytes: &[u8; rriv_board::EEPROM_SENSOR_SPECIAL_SETTINGS_SIZE]) -> Self {

    //     let special_config = GenericAnalogSpecialConfiguration::new_from_bytes(special_config_bytes);
    //     GenericAnalog {
    //         general_config,
    //         special_config
    //     }
    // }

    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: GenericAnalogSpecialConfiguration,
    ) -> Self {
        GenericAnalog {
            general_config,
            special_config,
            measured_parameter_values: [0.0; 2],
            m: 0_f64,
            b: 0_f64,
        }
    }
}

#[bitfield(u8)]
struct GenericAnalogDriverBitfield {
    #[bits(2)]
    adc_select: usize,

    #[bits(6)]
    unused: usize,
}

#[derive(Copy, Clone)]
pub struct GenericAnalogSpecialConfiguration {
    m: f32,                                // 4
    b: f32,                                // 4
    sensor_port: u8,                       // 1
    settings: GenericAnalogDriverBitfield, // 1
}

impl GenericAnalogSpecialConfiguration {
    pub fn parse_from_values(value: serde_json::Value) -> Result<GenericAnalogSpecialConfiguration, &'static str> {
        // should we return a Result object here? because we are parsing?  parse_from_values?
        let mut sensor_port: u8 = 0;
        match &value["sensor_port"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<u8, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            sensor_port = number;
                        }
                        Err(_) => return Err("invalid number"),
                    }
                }
            }
            _ => {
                return Err("missing sensor port")
            }
        }

        let mut bitfield = GenericAnalogDriverBitfield::new();
        match &value["adc_select"] {
            serde_json::Value::String(string) => match string.as_str() {
                "internal" => {
                    bitfield.set_adc_select(0);
                }
                "external" => {
                    bitfield.set_adc_select(1);
                }
                _ => {
                    return Err("bad adc select string");
                }
            },
            _ => {
                return Err("missing adc selection")
            }
        }

        Ok(Self {
            m: 0_f32,
            b: 0_f32,
            sensor_port: sensor_port,
            settings: bitfield
        })
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> GenericAnalogSpecialConfiguration {
        defmt::println!("loading: {:X}", bytes);
        for i in 0..8 {
            defmt::println!("loading {:#b}", bytes[i]);
        }
        let settings: *const GenericAnalogSpecialConfiguration = bytes.as_ptr().cast::<GenericAnalogSpecialConfiguration>();
        let settings = unsafe { *settings };
        // defmt::println!("loading {:#b} {} {} {}", settings.b, settings.b, settings.b as f64, (settings.b as f64) );

        settings
    }
}
