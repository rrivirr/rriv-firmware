use core::num::Wrapping;

use crate::sensor_name_from_type_id;

use super::types::*;
use serde_json::json;

pub struct K30CO2 {
    general_config: SensorDriverGeneralConfiguration,
    special_config: K30CO2SpecialConfiguration,
    measured_parameter_values: [f64; 2],
    m: f64,
    b: f64,
}

impl SensorDriver for K30CO2 {
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
           "type" : sensor_name,
           "m": self.m, 
           "b" : self.b, 
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        self.m = self.special_config.m as f64;
        self.b = self.special_config.b as f64;
    }
 
    getters!();

    fn get_measured_parameter_count(&mut self) -> usize {
        if self.m != 0.0 && self.b != 0.0 {
            return 2;
        } else {
            return 1;
        }
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        return Ok(self.measured_parameter_values[index]);
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let mut identifier =  single_raw_or_cal_parameter_identifiers(index, None);
        let nul_range_end = identifier.iter()
        .position(|&c| c == b'\0')
        .unwrap_or(identifier.len());
        if nul_range_end <= identifier.len() - 5 {
            identifier[nul_range_end..nul_range_end + 4].copy_from_slice(b"_ppm");
        }
        identifier
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
       // send the i2c command
       const I2C_ADDRESS: u8 = 0x68;
       const READ_RAM_COMMAND: u8 = 0x2;
       const NUMBER_OF_BYTES: u8 = 2;
       const CO2_VALUE_ADDRESS: u8 = 0x08;

       const READ_COMPLETE_STATUS: u8 = 0x21;
        //    const READ_INCOMPLETE_STATUS: u8 = 0x20;
       

        let mut command: [u8; 4] = [0; 4];
        command[0] = (READ_RAM_COMMAND << 4) + NUMBER_OF_BYTES;
        command[1] = 0; // MSB of RAM address
        command[2] = CO2_VALUE_ADDRESS; // MSB of RAM address is 0;
        let checksum: Wrapping<u8> = Wrapping(command[0]) + Wrapping(command[1]) + Wrapping(command[2]);
        command[3] = checksum.0;
        defmt::println!("command {:X}", command);

        let mut response: [u8; 4] = [0; 4];
        self.measured_parameter_values[0] = -1 as f64;

        match board.ic2_write(I2C_ADDRESS, &command) {
            Ok(_) => defmt::println!("request sent ok"),
            Err(err) => {
                defmt::println!("i2c err: {:?}", err);
                self.measured_parameter_values[0] = -1 as f64;
                return;
            }
,
        }

        board.delay_ms(40); // 20ms is 'typical' in the datasheet.
        
        match board.ic2_read(I2C_ADDRESS, &mut response){
            Ok(_) => {
                defmt::println!("response {:X}", response);
                // check if read is complete
                if response[0] != READ_COMPLETE_STATUS { // read is incomplete
                    defmt::println!("Read incomplete");
                    return; // TODO: retry the read
                }

                // validate checksum
                let checksum: Wrapping<u8> = Wrapping(response[0]) + Wrapping(response[1]) + Wrapping(response[2]);
                if response[3] != checksum.0 {
                    defmt::println!("Invalid checksum");
                    return; // TODO: retry the read
                }

                // 2 bytes signed integer
                // MSB at lower address.  
                // response[1] MSB, response[2] LSB: Big Endian

                // Convert to i16
                let value_bytes: [u8; 2] = [response[1], response[2]];
                let value: u16 = u16::from_be_bytes(value_bytes); // datasheet claims this should be i16, but seem like this is (u16 / 100) ppm
                self.measured_parameter_values[0] = value as f64; // / 100_f64;
            },
            Err(err) => { 
                defmt::println!("i2c err: {:?}", err);
                self.measured_parameter_values[0] = -1 as f64
            },
        }
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

impl K30CO2 {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: K30CO2SpecialConfiguration,
    ) -> Self {
        K30CO2 {
            general_config,
            special_config,
            measured_parameter_values: [0.0; 2],
            m: 0_f64,
            b: 0_f64,
        }
    }
}

#[derive(Copy, Clone)]
pub struct K30CO2SpecialConfiguration {
    m: f32,                                // 4
    b: f32,                                // 4
}

impl K30CO2SpecialConfiguration {
    #[allow(unused)]
    pub fn parse_from_values(value: serde_json::Value) -> Result<K30CO2SpecialConfiguration, &'static str> {
        Ok( Self {
            m: 0_f32,
            b: 0_f32,
        } )
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> K30CO2SpecialConfiguration {
        defmt::println!("loading: {:X}", bytes);
        for i in 0..8 {
            defmt::println!("loading {:#b}", bytes[i]);
        }
        let settings: *const K30CO2SpecialConfiguration = bytes.as_ptr().cast::<K30CO2SpecialConfiguration>();
        let settings = unsafe { *settings };
        // defmt::println!("loading {:#b} {} {} {}", settings.b, settings.b, settings.b as f64, (settings.b as f64) );

        settings
    }
}
