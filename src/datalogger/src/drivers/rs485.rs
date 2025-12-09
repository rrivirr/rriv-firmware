// use modbus_core::{Data, RequestPdu, ResponsePdu, rtu::{Header, RequestAdu, ResponseAdu}};
// use serde_json::json;

// use super::types::*;

// // Define the 'registers' 
// // input registers
// // 0-23 : the up to 24 thermistor values, calibrated.

// // holding registers
// // timing register
// //      pwm duty cycle
// //      on time
// //      off time


// // one question is about when/how the mapping happens
// // since the different driver setups.  how do we know what driver is in what stop, and how to talk to its registers?
// // we'd need to have an encapsultation i guess.  we encapsulate an ADU for the sensor inside an ADU that goes to the device

// pub struct Rs485DataRequestDriver {
//     general_config: SensorDriverGeneralConfiguration,
//     special_config: EmptySpecialConfiguration,
// }

// impl SensorDriver for Rs485DataRequestDriver {

//     getters!();

//     fn get_configuration_json(&mut self) -> serde_json::Value {
//         let mut sensor_id = self.get_id();
//         let sensor_id = match util::str_from_utf8(&mut sensor_id) {
//             Ok(sensor_id) => sensor_id,
//             Err(_) => "Invalid",
//         };

//         json!({
//             "id" : sensor_id    
//         })
//     }

//     fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
//     }

//     fn get_measured_parameter_count(&mut self) -> usize {
//         return 7; // TODO: this needs access to the board, so it can count up the parameters
//     }

//     fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
//         // this needs access to the board, so it can count up..
//         // this isn't a sensor, it's a telemeter server
//         // this is a datalogger level this, we would never have muliple.
//     }

//     fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
//         todo!()
//     }

//     fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
//         todo!()
//     }
// }

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