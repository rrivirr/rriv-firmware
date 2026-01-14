use serde_json::json;

use crate::sensor_name_from_type_id;

use super::types::*;

#[derive(Copy, Clone)]
pub struct MCP9808TemperatureDriverSpecialConfiguration {
    calibration_offset: i16, // TODO: This needs to get stored into the EEPROM, and we don't that yet!
    address: u8,
}

impl MCP9808TemperatureDriverSpecialConfiguration {
    pub fn parse_from_values(value: serde_json::Value) -> Result<MCP9808TemperatureDriverSpecialConfiguration, &'static str>  
    {
        let mut address: u8 = 0b0011000; // default address
        match &value["address"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            address = match number {
                                0 => 0b0011000,
                                1 => 0b0011001,
                                2 => 0b0011010,
                                3 => 0b0011011,
                                4 => 0b0011100,
                                5 => 0b0011101,
                                6 => 0b0011110,
                                7 => 0b0011111,
                                _ => return Err("invalid address")
                            };
                        }
                        Err(_) => return Err("invalid on time")
                    }
                }
            }
            _ => {
                return Err("on time is required");
            }
        }

        Ok(Self {
            calibration_offset: 0,
            address: address,
        })
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> MCP9808TemperatureDriverSpecialConfiguration {
        let settings = bytes.as_ptr().cast::<MCP9808TemperatureDriverSpecialConfiguration>();
        unsafe { *settings } 
    }

    pub fn new(calibration_offset: i16) -> MCP9808TemperatureDriverSpecialConfiguration {
        Self {
            calibration_offset: calibration_offset,
            address: 0b0011000,
        }
    }

}

const NUMBER_OF_MEASURED_PARAMETERS : usize = 2;

pub struct MCP9808TemperatureDriver {
    general_config: SensorDriverGeneralConfiguration,
    special_config: MCP9808TemperatureDriverSpecialConfiguration,
    measured_parameter_values: [f64; NUMBER_OF_MEASURED_PARAMETERS],
    address: u8,
    calibration_offset: f64
}

impl SensorDriver for MCP9808TemperatureDriver {

    fn get_configuration_json(&mut self) -> serde_json::Value  {

        let sensor_name_bytes = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name_str = core::str::from_utf8(&sensor_name_bytes).unwrap_or_default();

        json!({ 
            "id" : self.get_id(),
            "type" : sensor_name_str,
            "calibration_offset": self.special_config.calibration_offset
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        self.calibration_offset = (self.special_config.calibration_offset as f64) / 1000_f64;
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

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8;16] {
        return single_raw_or_cal_parameter_identifiers(index, Some(b'T'));
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        
        let message = [AMBIENT_TEMPERATURE_REGISTER_ADDRESS];
        let mut buffer: [u8; 2] = [0; 2];
        match board.ic2_write(self.address, &message ) {
            Ok(_) => {
                // continue
            }
            Err(_) =>  {
                // read error, improve this output later by returning a Result<> from take measurement
                self.measured_parameter_values[0] = f64::MAX;
                return;
            },

        }
        match board.ic2_read(self.address, &mut buffer) {
            Ok(_) => {
                // continue
            },
            Err(_) => {
                // read error, improve this output later by returning a Result<> from take measurement
                self.measured_parameter_values[0] = f64::MAX;
                return;
            },
        }

        //Convert the temperature data
        //First Check flag bits
        // follows from https://ww1.microchip.com/downloads/en/DeviceDoc/MCP9808-0.5C-Maximum-Accuracy-Digital-Temperature-Sensor-Data-Sheet-DS20005095B.pdf
        let mut upper_byte: u16 = buffer[0].into();
        let lower_byte: u16 = buffer[1].into();
        if (upper_byte & 0x80) == 0x80 { //T A ≥ TCRIT
        }
        if (upper_byte & 0x40) == 0x40 { //T A > TUPPER
        }
        if (upper_byte & 0x20) == 0x20 { //T A < TLOWER
        }

        upper_byte = upper_byte & 0x1F; //Clear flag bits
        let temperature: f64 = 
            if (upper_byte & 0x10) == 0x10 { //T A < 0°Ca
                upper_byte = upper_byte & 0x0F;//Clear SIGN
                let upper_byte: f64 = upper_byte.into();
                let lower_byte: f64 = lower_byte.into();
                256.0 - (upper_byte * 16.0 + lower_byte / 16.0)
            } else { //T A ≥ 0°C

                let upper_byte: f64 = upper_byte.into();
                let lower_byte: f64 = lower_byte.into();
                upper_byte * 16.0 + lower_byte / 16.0
                //Temperature = Ambient Temperature (°C)
            };

        self.measured_parameter_values[0] = temperature;
        self.measured_parameter_values[1] = temperature + self.calibration_offset;

    }

    fn clear_calibration(&mut self) {
        self.calibration_offset = 0_f64;
        self.special_config.calibration_offset = 0_i16;
    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
       // validation
       defmt::println!("pairs len {:?}", pairs.len());
       if pairs.len() != 1 {
        return Err(());
       }

       //fit
       let single = & pairs[0];
       let point = single.point;
       let value = single.values[0];
       self.calibration_offset = point - value;
       self.special_config.calibration_offset = (self.calibration_offset  * 1000_f64) as i16;
       defmt::println!("fit {}", self.special_config.calibration_offset);    
       Ok(())
    }
        
}

const AMBIENT_TEMPERATURE_REGISTER_ADDRESS: u8 = 0x05;

impl MCP9808TemperatureDriver {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: MCP9808TemperatureDriverSpecialConfiguration,
    ) -> Self {
        MCP9808TemperatureDriver {
            general_config,
            special_config,
            measured_parameter_values: [0.0; NUMBER_OF_MEASURED_PARAMETERS],
            address: special_config.address,
            calibration_offset: 0_f64 // default value, can be calculated from special_config
        }
    }

    pub fn new_with_address(
        general_config: SensorDriverGeneralConfiguration,
        special_config: MCP9808TemperatureDriverSpecialConfiguration,
        address: u8
    ) -> Self {
        MCP9808TemperatureDriver {
            general_config,
            special_config,
            measured_parameter_values: [0.0; NUMBER_OF_MEASURED_PARAMETERS],
            address: address,
            calibration_offset: 0_f64
        }
    }

    pub fn get_calibration_offset(&self) -> &i16 {
        return &self.special_config.calibration_offset;
    }

}