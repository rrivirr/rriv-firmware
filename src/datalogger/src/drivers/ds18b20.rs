pub const CONVERT_TEMP: u8 = 0x44;
pub const WRITE_SCRATCHPAD: u8 = 0x4E;
pub const READ_SCRATCHPAD: u8 = 0xBE;
pub const COPY_SCRATCHPAD: u8 = 0x48;
pub const RECALL_EEPROM: u8 = 0xB8;

//use embedded_hal::blocking::delay::DelayMs;

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Resolution {
    Bits9 = 0b00011111,
    Bits10 = 0b00111111,
    Bits11 = 0b01011111,
    Bits12 = 0b01111111,
}

impl Resolution {
    pub fn max_measurement_time_millis(&self) -> u16 {
        match self {
            Resolution::Bits9 => 94,
            Resolution::Bits10 => 188,
            Resolution::Bits11 => 375,
            Resolution::Bits12 => 1000,
        }
    }

    /// Blocks for the amount of time required to finished measuring temperature
    /// using this resolution
    //    pub fn delay_for_measurement_time(&self, delay: &mut impl DelayMs<u16>) {
    //         delay.delay_ms(self.max_measurement_time_millis());
    //         }

    pub(crate) fn from_config_register(config: u8) -> Option<Resolution> {
        match config {
            0b00011111 => Some(Resolution::Bits9),
            0b00111111 => Some(Resolution::Bits10),
            0b01011111 => Some(Resolution::Bits11),
            0b01111111 => Some(Resolution::Bits12),
            _ => None,
        }
    }

    pub(crate) fn to_config_register(&self) -> u8 {
        *self as u8
    }
}

use rtt_target::rprintln;
use serde_json::json;
use util::any_as_u8_slice;

use crate::registry::sensor_name_from_type_id;

use super::types::*;

pub const EMPTY_SIZE: usize = 32;
pub const NUMBER_OF_MEASURED_PARAMETERS: usize = 2;

#[derive(Copy, Clone)]
pub struct Ds18b20SpecialConfiguration {
    // calibrate data?
    // power mode
    // resolution mode
    _empty: [u8; 32],
}

impl Ds18b20SpecialConfiguration {
    pub fn new_from_values(value: serde_json::Value) -> Ds18b20SpecialConfiguration {
        Self {
            _empty: [b'\0'; EMPTY_SIZE],
        }
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> Ds18b20SpecialConfiguration {
        Self {
            _empty: [b'\0'; EMPTY_SIZE],
        }
    }
}

pub struct Ds18b20 {
    general_config: SensorDriverGeneralConfiguration,
    special_config: Ds18b20SpecialConfiguration,
    measured_parameter_values: [f64; NUMBER_OF_MEASURED_PARAMETERS],
}

impl Ds18b20 {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: Ds18b20SpecialConfiguration,
    ) -> Ds18b20 {
        Ds18b20 {
            general_config,
            special_config,
            measured_parameter_values: [0.0; NUMBER_OF_MEASURED_PARAMETERS],
        }
    }
}

impl SensorDriver for Ds18b20 {
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        // todo!()
    }

    getters!();

    fn get_measured_parameter_count(&mut self) -> usize {
        return NUMBER_OF_MEASURED_PARAMETERS;
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        if index < NUMBER_OF_MEASURED_PARAMETERS {
            Ok(self.measured_parameter_values[index])
        } else {
            return Err(());
            // return -1.0_f64;
        }
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let identifiers: [&str; NUMBER_OF_MEASURED_PARAMETERS] = ["T_raw", "T_cal"];
        let mut buf: [u8; 16] = [0_u8; 16];

        let mut identifier: &str = "invalid";
        if index <= NUMBER_OF_MEASURED_PARAMETERS {
            identifier = identifiers[index];
        }
        for i in 0..identifier.len() {
            buf[i] = identifier.as_bytes()[i];
        }
        return buf;
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
       // just read from a single device
       while true {
        board.disable_interrupts();

        // take measurement
        board.one_wire_reset();
        //onewire.reset(delay)?;
        board.one_wire_skip_address();
        //onewire.skip_address(delay)?;
        board.one_wire_write_byte(CONVERT_TEMP);
        //onewire.write_byte(commands::CONVERT_TEMP, delay)?;
        //delay.delay_ms(self.max_measurement_time_millis());
        //Resolution::Bits12.delay_for_measurement_time(delay));
        let delay_ms = Resolution::Bits12.max_measurement_time_millis();
        board.delay_ms(delay_ms);


        // read measurement
        board.one_wire_reset();
        board.one_wire_skip_address();
        board.one_wire_write_byte(READ_SCRATCHPAD);
        let mut scratchpad = [0; 9];
        board.one_wire_read_bytes(&mut scratchpad);
        board.enable_interrupts();
        rprintln!("tried");

        let resolution = if let Some(resolution) = Resolution::from_config_register(scratchpad[4]) {
            resolution
        } else {
            //    return Err(OneWireError::CrcMismatch);
            rprintln!("Problem reading resolution from scratchpad");
            return;
        };
        let raw_temp = u16::from_le_bytes([scratchpad[0], scratchpad[1]]);
        let temperature = match resolution {
            Resolution::Bits12 => (raw_temp as f32) / 16.0,
            Resolution::Bits11 => (raw_temp as f32) / 8.0,
            Resolution::Bits10 => (raw_temp as f32) / 4.0,
            Resolution::Bits9 => (raw_temp as f32) / 2.0,
        };
        rprintln!("Temp C: {}", temperature);
       }



        rprintln!("starting take measurement");
        //board.one_wire_send_command(CONVERT_TEMP, Some(&self.address), delay)?;
        //trigger simultaneous measurement
        board.one_wire_reset();
        //onewire.reset(delay)?;
        board.one_wire_skip_address();
        //onewire.skip_address(delay)?;
        board.one_wire_write_byte(CONVERT_TEMP);
        //onewire.write_byte(commands::CONVERT_TEMP, delay)?;
        //delay.delay_ms(self.max_measurement_time_millis());
        //Resolution::Bits12.delay_for_measurement_time(delay));
        let delay_ms = Resolution::Bits12.max_measurement_time_millis();
        board.delay_ms(delay_ms);


        // just read from a single device
        board.one_wire_reset();
        board.one_wire_skip_address();
        board.one_wire_write_byte(READ_SCRATCHPAD);
        let mut scratchpad = [0; 9];
        board.one_wire_read_bytes(&mut scratchpad);

        let resolution = if let Some(resolution) = Resolution::from_config_register(scratchpad[4]) {
            resolution
        } else {
            //    return Err(OneWireError::CrcMismatch);
            rprintln!("Problem reading resolution from scratchpad");
            return;
        };
        let raw_temp = u16::from_le_bytes([scratchpad[0], scratchpad[1]]);
        let temperature = match resolution {
            Resolution::Bits12 => (raw_temp as f32) / 16.0,
            Resolution::Bits11 => (raw_temp as f32) / 8.0,
            Resolution::Bits10 => (raw_temp as f32) / 4.0,
            Resolution::Bits9 => (raw_temp as f32) / 2.0,
        };
        rprintln!("Temp C: {}", temperature);

        // // iterate over all the devices, and report their temperature
                // rprintln!("start bus search take measurement");
        // board.one_wire_bus_start_search();
        // rprintln!("iterate");
        // loop {
        //     if let Some(device_address) = board.one_wire_bus_search() {
        //         rprintln!("found a device");
        //         // todo: fix this so that you can check family code
        //         // if device_address.family_code() != ds18b20::FAMILY_CODE {
        //         //     // skip other devices
        //         //     continue;
        //         // }
        //         // You will generally create the sensor once, and save it for later
        //         // let sensor = Ds18b20::new(device_address)?;

        //         // // contains the read temperature, as well as config info such as the resolution used
        //         // let sensor_data = sensor.read_data(one_wire_bus, delay)?;
        //         // writeln!(tx, "Device at {:?} is {}°C", device_address, sensor_data.temperature);

        //         board.one_wire_reset();
        //         board.one_wire_match_address(device_address);
        //         board.one_wire_write_byte(READ_SCRATCHPAD);

        //         let mut scratchpad = [0; 9];
        //         board.one_wire_read_bytes(&mut scratchpad);

        //         let resolution =
        //             if let Some(resolution) = Resolution::from_config_register(scratchpad[4]) {
        //                 resolution
        //             } else {
        //                 //    return Err(OneWireError::CrcMismatch);
        //                 rprintln!("Problem reading resolution from scratchpad");
        //                 return;
        //             };
        //         let raw_temp = u16::from_le_bytes([scratchpad[0], scratchpad[1]]);
        //         let temperature = match resolution {
        //             Resolution::Bits12 => (raw_temp as f32) / 16.0,
        //             Resolution::Bits11 => (raw_temp as f32) / 8.0,
        //             Resolution::Bits10 => (raw_temp as f32) / 4.0,
        //             Resolution::Bits9 => (raw_temp as f32) / 2.0,
        //         };
        //         rprintln!("Temp C: {}", temperature);
        //     } else {
        //         break;
        //     }
        // }
    }

    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
        // TODO: this can become a utility or macro function
        let generic_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&self.general_config) };
        let special_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&self.special_config) };

        // rprintln!("saving {:#b} {} {} {}", self.special_config.b, self.special_config.b, self.special_config.b as f64, (self.special_config.b as f64) / 1000_f64 );
        copy_config_into_partition(0, generic_settings_bytes, storage);
        copy_config_into_partition(1, special_settings_bytes, storage);
    }

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

    fn update_actuators(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        // rprintln!("not implemented");
    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
        todo!()
    }

    fn clear_calibration(&mut self) {
        // rprintln!("not implemented");
    }
}
