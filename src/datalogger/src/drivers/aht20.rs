use core::{f64::MAX, num::Wrapping};

use rtt_target::rprintln;
use serde_json::json;

use crate::sensor_name_from_type_id;

use super::types::*;


const AHTX0_I2CADDR_DEFAULT:u8  = 0x38;   ///< AHT default i2c address
const AHTX0_I2CADDR_ALTERNATE:u8  =  0x39; ///< AHT alternate i2c address
const AHTX0_CMD_CALIBRATE:u8  =  0xE1;     ///< Calibration command
const AHTX0_CMD_TRIGGER:u8  =  0xAC;       ///< Trigger reading command
const AHTX0_CMD_SOFTRESET:u8  =  0xBA;     ///< Soft reset command
const AHTX0_STATUS_BUSY:u8  =  0x80;       ///< Status bit for busy
const AHTX0_STATUS_CALIBRATED:u8  =  0x08; ///< Status bit for calibrated

#[derive(Copy, Clone)]
pub struct AHT20SpecialConfiguration {
    wait_time: usize,
    empty: [u8; 28],
}

impl AHT20SpecialConfiguration {
    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> AHT20SpecialConfiguration {
        let settings = bytes.as_ptr().cast::<AHT20SpecialConfiguration>();
        unsafe { *settings }
    }

    pub fn new_from_values(value: serde_json::Value) -> AHT20SpecialConfiguration {
        return Self {
            wait_time: 0,
            empty: [0; 28]
        }
    }
}

pub struct AHT20 {
    general_config: SensorDriverGeneralConfiguration,
    special_config: AHT20SpecialConfiguration,
    initialized: bool,
    humidity: f64,
    temperature: f64
}

pub struct ConfigurationPayload {}

impl AHT20 {
    fn get_status(board: &mut dyn rriv_board::SensorDriverServices) -> u8 {
        let mut buffer: [u8; 1] = [0; 1];
        match board.ic2_read(AHTX0_I2CADDR_DEFAULT, &mut buffer) {
            Ok(_) => return buffer[0],
            Err(_) => return 0xFF
        }
    }

    fn loop_until_ready(board: &mut dyn rriv_board::SensorDriverServices) {
        while (AHT20::get_status(board) & AHTX0_STATUS_BUSY) != 0 {
            rprintln!("AHT20 is busy");
            board.delay_ms(10);
        }
    }

    fn is_calibrated(board: &mut dyn rriv_board::SensorDriverServices) -> bool {
        if (AHT20::get_status(board) & AHTX0_STATUS_CALIBRATED) > 0 {
            return true;
        } else {
            return false;
        }
    }

    fn self_calibrate(board: &mut dyn rriv_board::SensorDriverServices){
        let cmd = [AHTX0_CMD_CALIBRATE, 0x08, 0x00];
        board.ic2_write(AHTX0_I2CADDR_DEFAULT, &cmd);

        AHT20::loop_until_ready(board);
    }
    
}

impl SensorDriver for AHT20 {

    fn get_configuration_json(&mut self) -> serde_json::Value  {

        let sensor_name_bytes = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name_str = core::str::from_utf8(&sensor_name_bytes).unwrap_or_default();

        json!({ 
            "id" : self.get_id(),
            "type" : sensor_name_str,
            "wait_time": self.special_config.wait_time
        })
    }

       
    // TODO: should setup take board as a param?
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        // 20ms startup time after power cycle.  
        // This is already handled by board startup times
        // board.delay_ms(20);

        // Soft Reset
        match board.ic2_write(AHTX0_I2CADDR_DEFAULT, &[AHTX0_CMD_SOFTRESET]) {
            Ok(_) => {},
            Err(err) => {
                rprintln!("Failed to setup AHTX0 {:?}", err);
                self.initialized = false;
                return;
            }
        }
        board.delay_ms(20);

        AHT20::loop_until_ready(board);

        if !AHT20::is_calibrated(board) {
            AHT20::self_calibrate(board);

            if !AHT20::is_calibrated(board) {
                rprintln!("Failed to calibrate AHTX0");
                self.initialized = false;
                return;
            }
        }




    }

    getters!();

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        let cmd: [u8; 3] = [AHTX0_CMD_TRIGGER, 0x33, 0];
        match board.ic2_write(AHTX0_I2CADDR_DEFAULT, &cmd) {
            Ok(_) => {},
            Err(err) => {
                rprintln!("Failed write to AHT20 {:?}", err);
                self.humidity = MAX;
                self.temperature = MAX
            },
        }

        AHT20::loop_until_ready(board);

        let mut data = [0_u8; 6];
        match board.ic2_read(AHTX0_I2CADDR_DEFAULT, &mut data){
            Ok(_) => {},
            Err(err) => {
                rprintln!("Failed write to AHT20 {:?}", err);
                self.humidity = MAX;
                self.temperature = MAX
            },
        }

        let mut h: Wrapping<u32> = Wrapping(data[1] as u32);
        h = h << 8;
        h = h | Wrapping(data[2] as u32);
        h = h << 4;
        h = h | (Wrapping(data[3] as u32) >> 4);
        self.humidity = (h.0 as f64 * 100.0) / 0x100000 as f64;
   
        let mut t: u32 = (data[3] & 0x0F) as u32;
        t = t << 8;
        t = t | data[4] as u32;
        t = t << 8;
        t = t | data [5] as u32;
        self.temperature = t as f64 * 200.0 / (0x100000 as f64) - 50.0;

    }

    fn get_measured_parameter_count(&mut self) -> usize {
        2
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        match index {
            0 => Ok(self.humidity),
            1 => Ok(self.temperature),
            _ => Err(())
        }
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8;16] {
        let identifiers = ["humidity", "temperature"];
        let mut buf: [u8; 16] = [0; 16];
        if index > identifiers.len() {
            return buf;
        }

        let identifier = identifiers[index];
        buf[0..identifier.len()].copy_from_slice(identifier.as_bytes());
        return buf;

    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()>{
        let _ = pairs;
        Err(())
    }
    
    fn clear_calibration(&mut self) {
        rprintln!("not implemented");
    }
    
    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
        rprintln!("not implemented");
    }
       
    fn update_actuators(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
    }
}

impl AHT20 {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: AHT20SpecialConfiguration,
    ) -> Self {
        AHT20 {
            general_config,
            special_config,
            initialized: false,
            humidity: MAX,
            temperature: MAX
        }
    }
}
