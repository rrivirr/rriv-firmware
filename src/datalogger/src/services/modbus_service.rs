use alloc::boxed::Box;
use modbus_buffer::ModbusBuffer;
use rriv_board::{RRIVBoard, RXProcessor};


const USART_BUFFER_NUM: usize = 3; // Includes an extra empty cell for end marker
const USART_BUFFER_SIZE: usize = 50;

static BUFFER_SIZE: usize = 128usize;

pub fn setup(board: &mut impl RRIVBoard) {
    // not implemented, does nothing.  
    // see notes below
}


pub struct ModbusByteProcessor {
    modbus_buffer: ModbusBuffer<BUFFER_SIZE>,
    message: [u8; BUFFER_SIZE],
    pending_message_size: usize,
    new_bytes: bool
}

impl<'a> ModbusByteProcessor {
    pub fn new() -> ModbusByteProcessor {

        let modbus_buffer = ModbusBuffer::<BUFFER_SIZE>::new()
        .min_frame_len(3)
        .overwrite(true);

        ModbusByteProcessor {
            message: [u8::MAX; BUFFER_SIZE],
            pending_message_size: 0usize,
            modbus_buffer,
            new_bytes: false
        }
    }

    pub fn take_message(&mut self, board: &impl RRIVBoard) -> Result< modbus_core::rtu::ResponseAdu<'_>, ()> {

        //this needs to be in a critical section
        let mut buffer = [u8::MAX; BUFFER_SIZE];
        // defmt::println!("tryy decode {}", defmt::Debug2Format(&self.modbus_buffer));
        if let Some(message_size) = self.modbus_buffer.try_decode_frame(&mut buffer){
            self.message.copy_from_slice(&buffer[0..message_size]);
            self.pending_message_size = message_size;
        }

        if self.pending_message_size > 0 {
            match modbus_core::rtu::client::decode_response(&self.message){
                Ok(option) => {
                    match option {
                        Some(adu) => {
                            // TODO: can't just sent new bytes false here!
                            defmt::println!("got ADU");
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

    // TODO: we had 2 byte processors, we need to leak just one.  
    // actually maybe the setup function needs to take a ref?
    // this is not done!
    pub fn setup(&mut self, board: &mut impl RRIVBoard) {

        let byte_processor = Box::<&mut ModbusByteProcessor>::leak(Box::new(self));

        // pass a pointer to the leaked processor to Board::set_rx_processor
        // board.set_serial_rx_processor(rriv_board::SerialRxPeripheral::SerialPeripheral2, Box::new(byte_processor));s

    }
}




impl<'a, 'b> RXProcessor for ModbusByteProcessor {
    fn process_byte(&mut self, byte: u8) {
        defmt::println!("rx byte {:X}", byte);
        self.modbus_buffer.push(byte);
        self.new_bytes = true;
        defmt::println!("size: {}", self.modbus_buffer.len());

    }
}



