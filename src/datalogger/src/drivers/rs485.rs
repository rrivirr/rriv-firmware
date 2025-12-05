use modbus_core::{Data, RequestPdu, ResponsePdu, rtu::{Header, RequestAdu, ResponseAdu}};

pub struct Rs485Driver {
    // general_config: SensorDriverGeneralConfiguration,
    // special_config: RingTemperatureDriverSpecialConfiguration,
}

impl Rs485Driver {
    pub fn send_request(){

        let hdr = Header {
            slave: 1
        };

        let pdu = RequestPdu(modbus_core::Request::ReadInputRegisters(0, 1));

        let adu = RequestAdu {
            hdr,
            pdu
        };

        let mut buffer: [u8; _] = [0u8; 1 + 1 + 4 + 1];
        // 1 address + 1 function + 2 start req + 2 number of reg + 1 CRC

        match modbus_core::rtu::client::encode_request(adu, &mut buffer) {
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    pub fn send_response(){

         let hdr = Header {
            slave: 1
        };

        let data = match Data::from_words(words, target) {
            Ok(data) => data,
            Err(_) => todo!(),
        };
    

        let pdu = ResponsePdu(Ok(modbus_core::Response::ReadInputRegisters(data)));

        let adu = ResponseAdu {
            hdr,
            pdu
        };
        
        match modbus_core::rtu::server::encode_response(adu, &mut buffer) {
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    pub fn decode_response(){
        // get message from uart5

        let mut buf: [u8; _] = [0u8; 1 + 1 + 4 + 1]; // need to know how long
        match modbus_core::rtu::client::decode_response(buf){
            Ok(option) => {
                match option {
                    Some(adu) => {
                        adu.hdr;
                        adu.pdu;
                    },
                    None => todo!(),
                }
                
            },
            Err(err) => todo!(),
        }
    }
}