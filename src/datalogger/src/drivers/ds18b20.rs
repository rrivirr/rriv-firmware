pub const CONVERT_TEMP: u8 = 0x44;
pub const _WRITE_SCRATCHPAD: u8 = 0x4E;
pub const READ_SCRATCHPAD: u8 = 0xBE;
pub const _COPY_SCRATCHPAD: u8 = 0x48;
pub const _RECALL_EEPROM: u8 = 0xB8;

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
            Resolution::Bits12 => 750,
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

    #[allow(unused)]
    pub(crate) fn to_config_register(&self) -> u8 {
        *self as u8
    }
}

use rtt_target::rprintln;

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
    pub fn parse_from_values(_value: serde_json::Value) -> Result<Ds18b20SpecialConfiguration, &'static str> {
        Ok ( Self {
            _empty: [b'\0'; EMPTY_SIZE],
        })
    }

    pub fn new_from_bytes(
        _bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> Ds18b20SpecialConfiguration {
        Self {
            _empty: [b'\0'; EMPTY_SIZE],
        }
    }
}

pub struct Ds18b20 {
    general_config: SensorDriverGeneralConfiguration,
    #[allow(unused)]
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
    
    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        
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

        rprintln!("start bus search take measurement");

        // iterate over all the devices, and report their temperature
        board.one_wire_bus_start_search();
        loop {
            if let Some(device_address) = board.one_wire_bus_search() {
                // todo: fix this so that you can check family code
                // if device_address.family_code() != ds18b20::FAMILY_CODE {
                //     // skip other devices
                //     continue;
                // }
                // You will generally create the sensor once, and save it for later
                // let sensor = Ds18b20::new(device_address)?;

                // // contains the read temperature, as well as config info such as the resolution used
                // let sensor_data = sensor.read_data(one_wire_bus, delay)?;
                // writeln!(tx, "Device at {:?} is {}°C", device_address, sensor_data.temperature);

                board.one_wire_reset();
                board.one_wire_match_address(device_address);
                board.one_wire_write_byte(READ_SCRATCHPAD);

                let mut scratchpad = [0; 9];
                board.one_wire_read_bytes(&mut scratchpad);

                let resolution =
                    if let Some(resolution) = Resolution::from_config_register(scratchpad[4]) {
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
            } else {
                break;
            }
        }
    }
    

    fn get_configuration_json(&mut self) -> serde_json::Value {
        todo!()
    }
    
}
