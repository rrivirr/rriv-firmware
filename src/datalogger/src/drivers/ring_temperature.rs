// use alloc::fmt::format;

use crate::any_as_u8_slice;
use crate::sensor_name_from_type_id;

use super::mcp9808::*;

use super::types::*;
use alloc::boxed::Box;
use rtt_target::rprint;
use serde_json::json;

// TODO: calibration offsets for all 6 sensors need to be stored and loaded into this driver, and written to EEPROM.

#[derive(Copy, Clone)]
pub struct RingTemperatureDriverSpecialConfiguration {
    calibration_offset: [i16; 8], // 16
    address_offset: u8, // 1
    _reserved: [u8; 15], // 32 - 1 -16 = 15
}

impl RingTemperatureDriverSpecialConfiguration {
    pub fn new_from_values(_value: serde_json::Value) -> RingTemperatureDriverSpecialConfiguration {
        Self {
            calibration_offset: [0; 8],
            address_offset: 0,
            _reserved: [b'\0'; 15],
        } // Just using default address offset of 0 for now, need to optionally read from JSON
    }
    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> RingTemperatureDriverSpecialConfiguration {
        let settings = bytes
            .as_ptr()
            .cast::<RingTemperatureDriverSpecialConfiguration>();
        unsafe { *settings }
    }
}

const TEMPERATURE_SENSORS_ON_RING: usize = 6;

pub struct RingTemperatureDriver {
    general_config: SensorDriverGeneralConfiguration,
    special_config: RingTemperatureDriverSpecialConfiguration,
    measured_parameter_values: [f64; TEMPERATURE_SENSORS_ON_RING * 2],
    sensor_drivers: [MCP9808TemperatureDriver; TEMPERATURE_SENSORS_ON_RING],
}

impl RingTemperatureDriver {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: RingTemperatureDriverSpecialConfiguration,
    ) -> Self {
        let mut addresses: [u8; TEMPERATURE_SENSORS_ON_RING] = [
            0b0011000, 0b0011001, 0b0011010, 0b0011011, 0b0011100, 0b0011101,
        ];
        for i in 0..6 {
            addresses[i] = addresses[i] + special_config.address_offset;
        }

        RingTemperatureDriver {
            general_config,
            special_config,
            measured_parameter_values: [0.0; TEMPERATURE_SENSORS_ON_RING * 2],
            sensor_drivers: [
                MCP9808TemperatureDriver::new_with_address(
                    SensorDriverGeneralConfiguration::empty(),
                    MCP9808TemperatureDriverSpecialConfiguration::new(
                        special_config.calibration_offset[0],
                    ),
                    addresses[0],
                ),
                MCP9808TemperatureDriver::new_with_address(
                    SensorDriverGeneralConfiguration::empty(),
                    MCP9808TemperatureDriverSpecialConfiguration::new(
                        special_config.calibration_offset[1],
                    ),
                    addresses[1],
                ),
                MCP9808TemperatureDriver::new_with_address(
                    SensorDriverGeneralConfiguration::empty(),
                    MCP9808TemperatureDriverSpecialConfiguration::new(
                        special_config.calibration_offset[2],
                    ),
                    addresses[2],
                ),
                MCP9808TemperatureDriver::new_with_address(
                    SensorDriverGeneralConfiguration::empty(),
                    MCP9808TemperatureDriverSpecialConfiguration::new(
                        special_config.calibration_offset[3],
                    ),
                    addresses[3],
                ),
                MCP9808TemperatureDriver::new_with_address(
                    SensorDriverGeneralConfiguration::empty(),
                    MCP9808TemperatureDriverSpecialConfiguration::new(
                        special_config.calibration_offset[4],
                    ),
                    addresses[4],
                ),
                MCP9808TemperatureDriver::new_with_address(
                    SensorDriverGeneralConfiguration::empty(),
                    MCP9808TemperatureDriverSpecialConfiguration::new(
                        special_config.calibration_offset[5],
                    ),
                    addresses[5],
                ),
            ],
        }
    }
}




const INDEX_TO_BYTE_CHAR: [u8; TEMPERATURE_SENSORS_ON_RING] = [b'A', b'B', b'E', b'C', b'F', b'D'];


impl SensorDriver for RingTemperatureDriver {
    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
        // right now this just gets the bytes
        // but the special settings probably should be consisted as member variables and copied back to the storage struct

        let generic_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&self.general_config) };
        let special_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&self.special_config) };

        copy_config_into_partition(0, generic_settings_bytes, storage);
        copy_config_into_partition(1, special_settings_bytes, storage);
    }

    // TODO: this should come from a derived trait
    fn get_configuration_json(&mut self) -> serde_json::Value {
        // let sensor_id_str: [u8; 6] = core::str::from_utf8(self.get_id());

        let sensor_name_bytes = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name_str = core::str::from_utf8(&sensor_name_bytes).unwrap_or_default();

        json!({
            "id" : self.get_id(),
            "type" : sensor_name_str,
            "calibration_offset": self.special_config.calibration_offset
        })
    }

    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        for i in 0..TEMPERATURE_SENSORS_ON_RING {
            self.sensor_drivers[i].setup(board);
        }
    }

    getters!();

    fn get_measured_parameter_count(&mut self) -> usize {
        TEMPERATURE_SENSORS_ON_RING * 2
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        if self.measured_parameter_values[index] == f64::MAX {
            Err(())
        } else {
            Ok(self.measured_parameter_values[index])
        }
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let sensor_index = index / 2;
        let parameter_index = index % 2;
        let buf =
            self.sensor_drivers[sensor_index].get_measured_parameter_identifier(parameter_index);

        let mut buf2: [u8; 16] = [0; 16];
        buf2[0..10].copy_from_slice(&buf[0..10]);
        let mut end = buf2
                        .iter()
                        .position(|&x| x == b'\0')
                        .unwrap_or_else(|| 1);
        if end >= 14 { end = 14 }
        buf2[end] = INDEX_TO_BYTE_CHAR[sensor_index];
        buf2[end+1] = b'\0';
        return buf2;
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        for i in 0..TEMPERATURE_SENSORS_ON_RING {
            self.sensor_drivers[i].take_measurement(board);
            self.measured_parameter_values[i * 2] =
                match self.sensor_drivers[i].get_measured_parameter_value(0) {
                    Ok(value) => value,
                    Err(_) => f64::MAX,
                };
            self.measured_parameter_values[i * 2 + 1] =
                match self.sensor_drivers[i].get_measured_parameter_value(1) {
                    Ok(value) => value,
                    Err(_) => f64::MAX,
                };
        }
    }

    fn clear_calibration(&mut self) {
        for i in 0..self.sensor_drivers.len() {
            self.sensor_drivers[i].clear_calibration();
        }
    }
    
    fn update_actuators(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
        // validate
        if pairs.len() != 1 {
            return Err(());
        }

        if pairs[0].values.len() < 6 {
            // TODO: check for zeros, not for len().  len is constant
            rprint!("not enough values to calibrate");
            return Err(());
        }

        // fit
        let single = &pairs[0];
        let point = single.point;
        let values = &single.values;
        for i in 0..TEMPERATURE_SENSORS_ON_RING as usize {
            let pairs = CalibrationPair {
                point: point,
                values: Box::new([values[i]]),
            };
            let result = self.sensor_drivers[i].fit(&[pairs]);
            self.special_config.calibration_offset[i] = self.sensor_drivers[i].get_calibration_offset().clone();
            match result {
                Ok(_) => true,
                Err(_) => return Err(()),
            };
        }
        Ok(())
    }
}
