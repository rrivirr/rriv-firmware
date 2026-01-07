use core::time;

use modbus_core::{Data, ResponsePdu};
use modbus_core::rtu::{Header, ResponseAdu};
use rriv_board::RRIVBoard;

use alloc::boxed::Box;

use crate::drivers::resources::gpio::GpioRequest;
use crate::services::usart_service;
use crate::telemetry::telemeters::{MAX_DATA_VALUES, Telemeter, telemeter};


const RRIV_DEFAULT_MODBUS_CLIENT_ID : u8 = 24;

pub struct ModBusRTU();

impl Telemeter for ModBusRTU {

    fn transmit(&mut self, board: &mut dyn RRIVBoard, values: &[i16; MAX_DATA_VALUES] ) {
        send_input_registers_response(board, values);
    }
    
    fn run_loop_iteration(&mut self, board: &mut dyn RRIVBoard) {
        // nothing to do here yet
    }
    
    fn ready_to_transmit(&mut self, board: &mut dyn RRIVBoard) -> bool {
        return true;
    }
    
    fn process_events(&mut self, board: &mut dyn RRIVBoard) {
        // not handled yet
    }
    
    fn get_requested_gpios(&self) -> crate::drivers::resources::gpio::GpioRequest {
        let mut request = GpioRequest::none();
        request.use_rs485();
        return request;
    }

}



fn transmit(board: &mut dyn RRIVBoard, payload: &[u8]){
        board.get_sensor_driver_services().set_gpio_pin_mode(1, rriv_board::gpio::GpioMode::PushPullOutput);

        let _ = board.get_sensor_driver_services().write_gpio_pin(1, true);
        board.usart_send(payload);
        let _ = board.get_sensor_driver_services().write_gpio_pin(1, false);
    }
    

  fn send_input_registers_response(board: &mut dyn RRIVBoard, values: &[i16; MAX_DATA_VALUES]){

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
            for byte in message {
                defmt::println!("tx byte: {:X}", byte);
            }
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
