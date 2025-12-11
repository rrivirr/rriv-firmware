use core::time;

use modbus_core::{Data, ResponsePdu};
use modbus_core::rtu::{Header, ResponseAdu};
use rriv_board::RRIVBoard;

use alloc::boxed::Box;

use crate::services::usart_service;


pub const MAX_DATA_VALUES : usize = 16usize;
const RRIV_DEFAULT_MODBUS_CLIENT_ID : u8 = 24;

// pub struct Serial {
// }


// impl Serial {
//     pub fn run_loop_iteration(&mut self, board: &mut impl RRIVBoard) {
//         board.usart_send(format!("timestamp:{}", board.timestamp()).as_str());
//     }
// }

pub fn transmit(board: &mut impl RRIVBoard, payload: &[u8] ) {
    board.get_sensor_driver_services().set_gpio_pin_mode(1, rriv_board::gpio::GpioMode::PushPullOutput);

    let _ = board.get_sensor_driver_services().write_gpio_pin(1, true);
    for i in 0..payload.len() {
        let prepared_message = format_args!("{}\r\n", payload[i]);
        usart_service::format_and_send(board, prepared_message); // just  using the normal usart right now
    }
    let _ = board.get_sensor_driver_services().write_gpio_pin(1, false);
}

pub fn send_input_registers_response(board: &mut impl RRIVBoard, values: [i16; MAX_DATA_VALUES]){

    let mut registers = [u16::MAX; MAX_DATA_VALUES];
    for i in 0..MAX_DATA_VALUES {
        registers[i] = values[i] as u16;
    }

 
    let hdr = Header {
        slave: RRIV_DEFAULT_MODBUS_CLIENT_ID
    };

    let mut target_buffer = [u8::MAX; 100]; // Data tracks the number of works, just need enuf here
    let data = match Data::from_words(&registers, &mut target_buffer) {
        Ok(data) => data,
        Err(_) => todo!(),
    };


    let pdu = ResponsePdu(Ok(modbus_core::Response::ReadInputRegisters(data)));

    let adu = ResponseAdu {
        hdr,
        pdu
    };
        
    let mut adu_buffer = [u8::MAX; 100];
    match modbus_core::rtu::server::encode_response(adu, &mut adu_buffer) {
        Ok(length) => {
            let message: &[u8] = &adu_buffer[0..length];
            transmit(board, message);
        },
        Err(_) => {
            let exception_response = modbus_core::ExceptionResponse{
                function: modbus_core::FunctionCode::ReadInputRegisters,
                exception: modbus_core::Exception::ServerDeviceFailure
            };
            let pdu = modbus_core::ResponsePdu(Err(exception_response));
            let adu = ResponseAdu {
                hdr,
                pdu
            };
            match modbus_core::rtu::server::encode_response(adu, &mut adu_buffer) {
                Ok(length) => {
                    let message = &adu_buffer[0..length];
                    transmit(board, message);
                },
                Err(_) => {
                    defmt::println!("Failed to send modbus exception response");
                },
            }
        }
    }
}