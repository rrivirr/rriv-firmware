use core::borrow::BorrowMut;
use alloc::boxed::Box;
use modbus_buffer::ModbusBuffer;
use rriv_board::{RRIVBoard, RXProcessor};
use util::str_from_utf8;


const USART_BUFFER_NUM: usize = 3; // Includes an extra empty cell for end marker
const USART_BUFFER_SIZE: usize = 50;

static BUFFER_SIZE: usize = 64usize;



pub struct ModbusByteProcessor {
    modbus_buffer: ModbusBuffer<BUFFER_SIZE>,
    message: [u8; 64],
    pending_message_size: usize
}

impl<'a> ModbusByteProcessor {
    pub fn new() -> ModbusByteProcessor {

        let modbus_buffer = ModbusBuffer::<BUFFER_SIZE>::new()
        .min_frame_len(3)
        .overwrite(true);

        ModbusByteProcessor {
            message: [u8::MAX; 64],
            pending_message_size: 0usize,
            modbus_buffer
        }
    }

    pub fn take_message(&mut self, board: &impl RRIVBoard) -> Result< modbus_core::rtu::ResponseAdu<'_>, ()> {

        // this needs to be in a critical section
        if self.pending_message_size > 0 {
            match modbus_core::rtu::client::decode_response(&self.message){
                Ok(option) => {
                    match option {
                        Some(adu) => {
                            Ok(adu.clone())

                            // we got a message
                            // need to mark self.message as empty
                            // this probably needs to happen in a critical section
                        },
                        None => Err(()),
                    }
                    
                },
                Err(err) => Err(())
            }
        } else {
            Err(())
        }

    }


}

impl<'a, 'b> RXProcessor for ModbusByteProcessor {
    fn process_byte(&mut self, byte: u8) {
        if self.pending_message_size > 0 {
            // we have a pending message, can't accept any more in the buffer
            return;
        }
        self.modbus_buffer.push(byte);
        let mut buffer = [u8::MAX; BUFFER_SIZE];
        if let Some(message_size) = self.modbus_buffer.try_decode_frame(&mut buffer){
            self.message.copy_from_slice(&buffer[0..message_size]);
            self.pending_message_size = message_size;
        }
    }
}


pub fn setup(board: &mut impl RRIVBoard) {

    let byte_processor = Box::<ModbusByteProcessor>::leak(Box::new(ModbusByteProcessor::new()));

    // pass a pointer to the leaked processor to Board::set_rx_processor
    board.set_serial_rx_processor(rriv_board::SerialRxPeripheral::SerialPeripheral2, Box::new(byte_processor));
}

