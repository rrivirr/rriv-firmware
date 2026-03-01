use serde_json::json;

use crate::sensor_name_from_type_id;

use super::types::*;

#[derive(Copy, Clone)]
pub struct MHZ9041ADriverSpecialConfiguration {
    calibration_offset: i16, // stored as value * 1000 for fixed-point precision
    address: u8,
}

impl MHZ9041ADriverSpecialConfiguration {
    pub fn parse_from_values(value: serde_json::Value) -> Result<MHZ9041ADriverSpecialConfiguration, &'static str>  
    {
        let mut address: u8 = MHZ9041A_DEFAULT_I2C_ADDRESS;
        match &value["address"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<u8, _> = number.try_into();
                    match number {
                        Ok(addr) => {
                            if addr < 0x03 || addr > 0x7F {
                                return Err("invalid address: must be 0x03..0x7F");
                            }
                            address = addr;
                            defmt::println!("MHZ9041A: configured with address=0x{:02X} from JSON", address);
                        }
                        Err(_) => return Err("invalid address value")
                    }
                }
            }
            _ => {
                defmt::println!("MHZ9041A: no address in config, using default 0x{:02X}", address);
            }
        }

        Ok(Self {
            calibration_offset: 0,
            address,
        })
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> MHZ9041ADriverSpecialConfiguration {
        let settings = bytes.as_ptr().cast::<MHZ9041ADriverSpecialConfiguration>();
        unsafe { *settings }
    }

    pub fn new(calibration_offset: i16) -> MHZ9041ADriverSpecialConfiguration {
        Self {
            calibration_offset,
            address: MHZ9041A_DEFAULT_I2C_ADDRESS,
        }
    }
}

/// Number of measured output parameters:
///   [0] = raw CH4 concentration (%LEL)
///   [1] = calibrated CH4 concentration (%LEL)
///   [2] = ambient temperature (°C)
///   [3] = fault code (0 = normal)
const NUMBER_OF_MEASURED_PARAMETERS: usize = 4;

pub struct MHZ9041ADriver {
    general_config: SensorDriverGeneralConfiguration,
    special_config: MHZ9041ADriverSpecialConfiguration,
    measured_parameter_values: [f64; NUMBER_OF_MEASURED_PARAMETERS],
    address: u8,
    calibration_offset: f64,
}

impl SensorDriver for MHZ9041ADriver {

    fn get_configuration_json(&mut self) -> serde_json::Value {

        let sensor_name_bytes = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name_str = core::str::from_utf8(&sensor_name_bytes).unwrap_or_default();

        json!({
            "id" : self.get_id(),
            "type" : sensor_name_str,
            "calibration_offset": self.special_config.calibration_offset,
            "address": self.special_config.address
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        defmt::println!("MHZ9041A: setup starting, address=0x{:02X}", self.address);
        self.calibration_offset = (self.special_config.calibration_offset as f64) / 1000_f64;

        // Set sensor to passive (polling) mode
        // Register 0x11 = REG_MODE, value 0x00 = passive mode
        let set_mode_cmd = [REG_MODE, MODE_PASSIVE];
        match board.ic2_write(self.address, &set_mode_cmd) {
            Ok(_) => {
                defmt::println!("MHZ9041A: passive mode set successfully");
            }
            Err(_) => {
                defmt::println!("MHZ9041A: FAILED to set passive mode");
            }
        }
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

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        match index {
            0 => {
                // Raw CH4 concentration
                let mut id = [0u8; 16];
                let tag = b"CH4r";
                id[..tag.len()].copy_from_slice(tag);
                id
            }
            1 => {
                // Calibrated CH4 concentration
                let mut id = [0u8; 16];
                let tag = b"CH4c";
                id[..tag.len()].copy_from_slice(tag);
                id
            }
            2 => {
                // Ambient temperature from the MHZ9041A module
                let mut id = [0u8; 16];
                let tag = b"CH4T";
                id[..tag.len()].copy_from_slice(tag);
                id
            }
            3 => {
                // Fault code
                let mut id = [0u8; 16];
                let tag = b"CH4e";
                id[..tag.len()].copy_from_slice(tag);
                id
            }
            _ => {
                let mut id = [0u8; 16];
                id[0] = b'?';
                id
            }
        }
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {

        defmt::println!("MHZ9041A: take_measurement starting, address=0x{:02X}", self.address);

        // --- Read CH4 concentration ---
        // Write the register address for LEL high byte
        let reg_lel = [REG_LEL_H];
        let mut lel_buffer: [u8; 2] = [0; 2];
        match board.ic2_write(self.address, &reg_lel) {
            Ok(_) => {
                defmt::println!("MHZ9041A: LEL register write OK");
            }
            Err(_) => {
                defmt::println!("MHZ9041A: LEL register write FAILED");
                self.measured_parameter_values[0] = f64::MAX;
                self.measured_parameter_values[1] = f64::MAX;
                self.measured_parameter_values[2] = f64::MAX;
                self.measured_parameter_values[3] = f64::MAX;
                return;
            }
        }
        match board.ic2_read(self.address, &mut lel_buffer) {
            Ok(_) => {
                defmt::println!("MHZ9041A: LEL read OK, raw bytes: [0x{:02X}, 0x{:02X}]", lel_buffer[0], lel_buffer[1]);
            }
            Err(_) => {
                defmt::println!("MHZ9041A: LEL read FAILED");
                self.measured_parameter_values[0] = f64::MAX;
                self.measured_parameter_values[1] = f64::MAX;
                self.measured_parameter_values[2] = f64::MAX;
                self.measured_parameter_values[3] = f64::MAX;
                return;
            }
        }

        // CH4 concentration: 2 bytes big-endian, raw value / 100.0 = %LEL
        // Round to nearest 0.1 %LEL as per the reference driver
        let raw_lel: u16 = ((lel_buffer[0] as u16) << 8) | (lel_buffer[1] as u16);
        let raw_lel_rounded: u16 = (raw_lel + 5) / 10 * 10;
        let ch4_concentration: f64 = (raw_lel_rounded as f64) / 100.0;

        defmt::println!("MHZ9041A: CH4 raw={}, rounded={}, concentration={}", raw_lel, raw_lel_rounded, ch4_concentration);

        self.measured_parameter_values[0] = ch4_concentration;
        self.measured_parameter_values[1] = ch4_concentration + self.calibration_offset;

        // --- Read ambient temperature ---
        let reg_temp = [REG_TEMP_H];
        let mut temp_buffer: [u8; 2] = [0; 2];
        match board.ic2_write(self.address, &reg_temp) {
            Ok(_) => {
                defmt::println!("MHZ9041A: TEMP register write OK");
            }
            Err(_) => {
                defmt::println!("MHZ9041A: TEMP register write FAILED");
                self.measured_parameter_values[2] = f64::MAX;
                self.measured_parameter_values[3] = f64::MAX;
                return;
            }
        }
        match board.ic2_read(self.address, &mut temp_buffer) {
            Ok(_) => {
                defmt::println!("MHZ9041A: TEMP read OK, raw bytes: [0x{:02X}, 0x{:02X}]", temp_buffer[0], temp_buffer[1]);
            }
            Err(_) => {
                defmt::println!("MHZ9041A: TEMP read FAILED");
                self.measured_parameter_values[2] = f64::MAX;
                self.measured_parameter_values[3] = f64::MAX;
                return;
            }
        }

        // Temperature: 2 bytes big-endian, raw value / 100.0 = °C
        let raw_temp: u16 = ((temp_buffer[0] as u16) << 8) | (temp_buffer[1] as u16);
        let temperature: f64 = (raw_temp as f64) / 100.0;
        defmt::println!("MHZ9041A: temperature raw={}, celsius={}", raw_temp, temperature);
        self.measured_parameter_values[2] = temperature;

        // --- Read fault/error code ---
        let reg_err = [REG_ERROR_CODE];
        let mut err_buffer: [u8; 1] = [0; 1];
        match board.ic2_write(self.address, &reg_err) {
            Ok(_) => {
                defmt::println!("MHZ9041A: ERROR register write OK");
            }
            Err(_) => {
                defmt::println!("MHZ9041A: ERROR register write FAILED");
                self.measured_parameter_values[3] = f64::MAX;
                return;
            }
        }
        match board.ic2_read(self.address, &mut err_buffer) {
            Ok(_) => {
                defmt::println!("MHZ9041A: ERROR read OK, fault_code=0x{:02X}", err_buffer[0]);
            }
            Err(_) => {
                defmt::println!("MHZ9041A: ERROR read FAILED");
                self.measured_parameter_values[3] = f64::MAX;
                return;
            }
        }
        self.measured_parameter_values[3] = err_buffer[0] as f64;

        defmt::println!("MHZ9041A: measurement complete, CH4={}, CH4_cal={}, temp={}, fault={}",
            self.measured_parameter_values[0],
            self.measured_parameter_values[1],
            self.measured_parameter_values[2],
            self.measured_parameter_values[3]
        );
    }

    fn clear_calibration(&mut self) {
        self.calibration_offset = 0_f64;
        self.special_config.calibration_offset = 0_i16;
    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
        defmt::println!("pairs len {:?}", pairs.len());
        if pairs.len() != 1 {
            return Err(());
        }

        let single = &pairs[0];
        let point = single.point;         // known reference value
        let value = single.values[0];     // measured raw CH4 concentration
        self.calibration_offset = point - value;
        self.special_config.calibration_offset = (self.calibration_offset * 1000_f64) as i16;
        defmt::println!("fit {}", self.special_config.calibration_offset);
        Ok(())
    }
}

// ─── I2C register addresses ────────────────────────────────────────────────────
// From the DFRobot MHZ9041A datasheet / reference Arduino driver

/// Default I2C address for the DFRobot MHZ9041A breakout board
const MHZ9041A_DEFAULT_I2C_ADDRESS: u8 = 0x75;

/// Vendor ID high byte register (used to verify communication)
#[allow(dead_code)]
const REG_VID_H: u8 = 0x02;

/// Expected value of VID_H register
#[allow(dead_code)]
const VID_H_EXPECTED: u8 = 0x33;

/// Device address register (read/write, 1 byte)
#[allow(dead_code)]
const REG_DEVICE_L: u8 = 0x05;

/// Baud rate register (for UART mode, 1 byte)
#[allow(dead_code)]
const REG_BAUD_L: u8 = 0x07;

/// Firmware version high byte register
#[allow(dead_code)]
const REG_VERSION_H: u8 = 0x0A;

/// CH4 concentration register, high byte (2 bytes big-endian, raw / 100.0 = %LEL)
const REG_LEL_H: u8 = 0x0C;

/// Ambient temperature register, high byte (2 bytes big-endian, raw / 100.0 = °C)
const REG_TEMP_H: u8 = 0x0E;

/// Error/fault code register (1 byte)
/// 0x00 = normal, see FaultCode enum for other values
const REG_ERROR_CODE: u8 = 0x10;

/// Working mode register (1 byte)
/// 0x00 = passive (polling), 0x01 = active (auto-reporting)
const REG_MODE: u8 = 0x11;

/// Reset register — write 0x01 to trigger sensor reset (takes ~2s)
#[allow(dead_code)]
const REG_RESET: u8 = 0x12;

/// Length of raw source data string
#[allow(dead_code)]
const REG_SOURCE_LEN: u8 = 0x20;

/// Raw source data string (variable length, up to 43 bytes)
#[allow(dead_code)]
const REG_SOURCE_DATA: u8 = 0x21;

/// Passive mode value for REG_MODE
const MODE_PASSIVE: u8 = 0x00;

/// Active mode value for REG_MODE
#[allow(dead_code)]
const MODE_ACTIVE: u8 = 0x01;


// ─── Fault code constants (for reference / future use) ─────────────────────────
// These correspond to the eFaultCode_t enum in the reference driver.
// They are exposed here for downstream code that may want to interpret
// measured_parameter_values[3].

#[allow(dead_code)]
pub const FAULT_NORMAL: u8 = 0x00;
#[allow(dead_code)]
pub const FAULT_TEMP_CONTROL_ERROR: u8 = 0x01;
#[allow(dead_code)]
pub const FAULT_AMBIENT_TEMP_ERROR: u8 = 0x02;
#[allow(dead_code)]
pub const FAULT_AMBIENT_AND_TEMP_CONTROL: u8 = 0x03;
#[allow(dead_code)]
pub const FAULT_LASER_SIGNAL_WEAK: u8 = 0x04;
#[allow(dead_code)]
pub const FAULT_AMBIENT_AND_SIGNAL_WEAK: u8 = 0x06;
#[allow(dead_code)]
pub const FAULT_LASER_SIGNAL_ERROR: u8 = 0x10;
#[allow(dead_code)]
pub const FAULT_AMBIENT_AND_SIGNAL_ERROR: u8 = 0x12;


// ─── Constructor ────────────────────────────────────────────────────────────────

impl MHZ9041ADriver {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: MHZ9041ADriverSpecialConfiguration,
    ) -> Self {
        MHZ9041ADriver {
            general_config,
            special_config,
            measured_parameter_values: [0.0; NUMBER_OF_MEASURED_PARAMETERS],
            address: special_config.address,
            calibration_offset: 0_f64,
        }
    }

    pub fn new_with_address(
        general_config: SensorDriverGeneralConfiguration,
        special_config: MHZ9041ADriverSpecialConfiguration,
        address: u8,
    ) -> Self {
        MHZ9041ADriver {
            general_config,
            special_config,
            measured_parameter_values: [0.0; NUMBER_OF_MEASURED_PARAMETERS],
            address,
            calibration_offset: 0_f64,
        }
    }

    pub fn get_calibration_offset(&self) -> &i16 {
        &self.special_config.calibration_offset
    }
}
