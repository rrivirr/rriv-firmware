use crate::sensor_name_from_type_id;

// use crate::drivers::atlas::*;

use super::types::*;
use bitfield_struct::bitfield;
use alloc::boxed::Box;
use rtt_target::{rprint, rprintln};
use serde::de::value;
use serde_json::json;

// Constants for EC_OEM register data
pub const DEVICE_TYPE: u8 = 0x00;
pub const FIRMWARE_VERSION: u8 = 0x01;
pub const ADDR_LOCK: u8 = 0x02;
pub const NEW_ADDR_REGISTER: u8 = 0x03;
pub const INT_CTRL: u8 = 0x04;
pub const LED_CTRL: u8 = 0x05;
pub const SLEEP_CTRL: u8 = 0x06;
pub const DATA_AVAILABLE: u8 = 0x07;

// Constants for LED mode
pub const LED_BLINK_ON_MEASUREMENT: u8 = 1;
pub const LED_OFF: u8 = 0;

const ATLAS_EC_DEFAULT_ADDRESS: u8 = 0x64;

#[derive(Copy, Clone, Debug)]
pub struct AtlasECSpecialConfiguration {
}

impl AtlasECSpecialConfiguration {
    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> AtlasECSpecialConfiguration {
        let settings = bytes.as_ptr().cast::<AtlasECSpecialConfiguration>();
        unsafe { *settings }
    }

    pub fn parse_from_values(_value: serde_json::Value) -> Result<AtlasECSpecialConfiguration, &'static str> {
        Ok( Self {
        })
    }
}

pub struct AtlasEC {
    general_config: SensorDriverGeneralConfiguration,
    special_config: AtlasECSpecialConfiguration,
    measured_parameter_values: [f64; 2],
}


impl SensorDriver for AtlasEC {
  
    fn get_configuration_json(&mut self) -> serde_json::Value {
        json!({ 
            "id" : util::str_from_utf8(&mut self.get_id()).unwrap_or_default()
        })
    }

    getters!();


    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {

        // Wake device just in cast
        let message: [u8; 2] = [SLEEP_CTRL, 1];
        board.ic2_write(ATLAS_EC_DEFAULT_ADDRESS, &message);

        // set LED mode
        let message: [u8; 2] = [LED_CTRL, 1];
        board.ic2_write(ATLAS_EC_DEFAULT_ADDRESS, &message);
    }

   
    fn get_measured_parameter_count(&mut self) -> usize {
        return 1;
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        Ok(self.measured_parameter_values[0])
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let mut identifier : [u8;16] = [0; 16];
        identifier[0] = b'e';
        identifier[1] = b'C';
        return identifier;
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        
        let message = [DATA_AVAILABLE];
        let mut available = [0u8];

        board.ic2_write_read(ATLAS_EC_DEFAULT_ADDRESS, &message, &mut available);
        if available[0] == 1 {
            let mut bytes = [u8::MAX, u8::MAX, u8::MAX, u8::MAX];
            let mut address = [0x18];
            for i in 0..4 {
                let mut buffer: [u8; 1] = [u8::MAX];
                let message: [u8; 1] = [0x18 + i];
                board.ic2_write_read(ATLAS_EC_DEFAULT_ADDRESS, &message, &mut buffer);
                bytes[i as usize] = buffer[0];
            }

            let value: u32 = bytes[3] as u32 + (bytes[2] as u32) << 8 + (bytes[1] as u32) << 16 + (bytes[0] as u32) << 24;
            rprintln!("value {}", value);
            let value: f64 = value as f64 / 1000f64;
            self.measured_parameter_values[0] = value;

            let message = [DATA_AVAILABLE, 0];
            board.ic2_write(ATLAS_EC_DEFAULT_ADDRESS, &message);
        } else {
            self.measured_parameter_values[0] = f64::MAX;
        }
    }
}


impl AtlasEC {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: AtlasECSpecialConfiguration,
    ) -> Self {
        AtlasEC {
            general_config,
            special_config,
            measured_parameter_values: [0.0; 2]
        }
    }
}