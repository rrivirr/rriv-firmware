use modbus_core::{Data, RequestPdu, ResponsePdu, rtu::{Header, RequestAdu, ResponseAdu}};
use serde_json::json;

use super::types::*;

// Define the 'registers' 
// input registers
// 0-23 : the up to 24 thermistor values, calibrated.

// holding registers
// timing register
//      pwm duty cycle
//      on time
//      off time


// one question is about when/how the mapping happens
// since the different driver setups.  how do we know what driver is in what stop, and how to talk to its registers?
// we'd need to have an encapsultation i guess.  we encapsulate an ADU for the sensor inside an ADU that goes to the device

pub struct ModbusDriverSpecialConfiguration {
    measured_parameter_count: usize,
}

impl ModbusDriverSpecialConfiguration {
    pub fn new() -> Self {
        Self {
            measured_parameter_count: 0,
        }
    }

    pub fn parse_from_values(value: serde_json::Value) -> Result<ModbusDriverSpecialConfiguration, &'static str> {
        Ok(ModbusDriverSpecialConfiguration::new())
    }

    pub fn new_from_bytes(
        _bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> ModbusDriverSpecialConfiguration {
       ModbusDriverSpecialConfiguration::new()
    }
}

pub struct ModbusDriver {
    general_config: SensorDriverGeneralConfiguration,
    special_config: ModbusDriverSpecialConfiguration,
    measured_parameter_values: [i16; 12]

}

impl ModbusDriver {
     pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: ModbusDriverSpecialConfiguration,
    ) -> Self {
        Self {
            general_config,
            special_config,
            measured_parameter_values: [0_i16; 12],
        }
    }
}

impl SensorDriver for ModbusDriver {

    getters!();

    fn get_configuration_json(&mut self) -> serde_json::Value {
        let mut sensor_id = self.get_id();
        let sensor_id = match util::str_from_utf8(&mut sensor_id) {
            Ok(sensor_id) => sensor_id,
            Err(_) => "Invalid",
        };

        json!({
            "id" : sensor_id    
        })
    }

    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
    }

    fn get_measured_parameter_count(&mut self) -> usize {
        return self.special_config.measured_parameter_count; // TODO: take_measurement will update the number of parameters being returns
                  // or setup will do so, in addition to getting the parameter identifiers / types
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        // this needs access to the board, so it can count up..
        // this isn't a sensor, it's a telemeter server
        // this is a datalogger level this, we would never have muliple.
        // naw, it's a driver... because we might have different drivers (clients) for different modbus servers.  and even do customizations
        if index > self.special_config.measured_parameter_count {
            return Err(());
        }
        Ok(self.measured_parameter_values[index] as f64 / 10.0)
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let mut buffer = [0u8; 16];
        buffer[0..6].copy_from_slice("value_".as_bytes());
        let c = char::from_digit(index as u32, 10).unwrap();
        let a = c as u8;
        buffer[6] = a;
        buffer

    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
       // send requests to modbus sensors
    }

    fn receive_modbus(&mut self, adu: modbus_core::rtu::ResponseAdu) {
        // we got a modbus response
        let hdr = adu.hdr;
        let pdu = adu.pdu;

        match pdu.0 {
            Ok(response) => {
                match response {
                    modbus_core::Response::ReadInputRegisters(data) => {
                        self.special_config.measured_parameter_count = data.len();
                        for i in 0..data.len() {
                            self.measured_parameter_values[i] =match data.get(i){
                                Some(value) => value as i16,
                                None => i16::MAX,
                            }
                        }
                    },
                    _ => {
                        // not doing anything else
                        defmt::println!("wrong function")
                    }
                }
            },
            Err(_exception) => {
                defmt::println!("modbus exception")
            }
        }

        
    }
}

// impl Rs485DataRequestDriver {
//     pub fn send_request(){

//         let hdr = Header {
//             slave: 1 //the address of the modbus device (can be the address of the 3d ring temperature sensor, for instance. slot number)
//         };

//         let pdu = RequestPdu(modbus_core::Request::ReadInputRegisters(0, 23)); // read from the 3d ring temperature sensor 

//         let sensor_adu = RequestAdu {
//             hdr,
//             pdu
//         };

//         let mut buffer: [u8; _] = [0u8; 1 + 1 + 4 + 1];
//         // 1 address + 1 function + 2 start req + 2 number of reg + 1 CRC

//         match modbus_core::rtu::client::encode_request(sensor_adu, &mut buffer) {
//             Ok(payload) => board.send_serial2(payload),
//             Err(_) => todo!(),
//         }

//          match modbus_core::rtu::client::encode_request(sensor_adu, &mut buffer) {
//             Ok(payload) => {
//                 let pdu = RequestPdu(modbus_core::Request::Custom(modbus_core::FunctionCode::Custom(0xFF), &payload));
//                 let hdr = Header {
//                     slave: 100 // the address of the rriv board
//                 };
//                 let adu = RequestAdu {
//                     hdr,
//                     pdu
//                 };

//             },
//             Err(_) => todo!(),
//         }
//     }

//     pub fn send_response(){

//          let hdr = Header {
//             slave: 1
//         };

//         let data = match Data::from_words(words, target) {
//             Ok(data) => data,
//             Err(_) => todo!(),
//         };
    

//         let pdu = ResponsePdu(Ok(modbus_core::Response::ReadInputRegisters(data)));

//         let adu = ResponseAdu {
//             hdr,
//             pdu
//         };
        
//         match modbus_core::rtu::server::encode_response(adu, &mut buffer) {
//             Ok(_) => todo!(),
//             Err(_) => todo!(),
//         }
//     }

//     pub fn decode_response(){
//         // get message from uart5

//         let mut buf: [u8; _] = [0u8; 1 + 1 + 4 + 1]; // need to know how long
//         match modbus_core::rtu::client::decode_response(buf){
//             Ok(option) => {
//                 match option {
//                     Some(adu) => {
//                         adu.hdr;
//                         adu.pdu;
//                     },
//                     None => todo!(),
//                 }
                
//             },
//             Err(err) => todo!(),
//         }
//     }
// }

            // measured_parameters_data: [i16::MAX; 12]
// 