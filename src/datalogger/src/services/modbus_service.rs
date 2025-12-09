use core::borrow::BorrowMut;
use alloc::boxed::Box;
use modbus_buffer::ModbusBuffer;
use rriv_board::{RRIVBoard, RXProcessor};
use rtt_target::rprintln;
use util::str_from_utf8;


const USART_BUFFER_NUM: usize = 3; // Includes an extra empty cell for end marker
const USART_BUFFER_SIZE: usize = 50;

static BUFFER_SIZE: usize = 64usize;
static mut MODBUS_BUFFER : Option<ModbusBuffer<BUFFER_SIZE>> = None;



pub struct ModbusByteProcessor {
    message: [u8; 64]
}

impl<'a> ModbusByteProcessor {
    pub fn new() -> ModbusByteProcessor {

        let modbus_buffer = ModbusBuffer::<BUFFER_SIZE>::new()
        .min_frame_len(3)
        .overwrite(true);

        ModbusByteProcessor {
            message: [u8::MAX; 64]
        }
    }
}

impl<'a, 'b> RXProcessor for ModbusByteProcessor {
    fn process_byte(&mut self, byte: u8) {
        unsafe { 
            MODBUS_BUFFER.as_mut().unwrap().push(byte);
            let mut buffer = [u8::MAX; BUFFER_SIZE];
            let message_size = MODBUS_BUFFER.as_mut().unwrap().try_decode_frame(&mut buffer);

            // can't copy it into the struct becawe we are not mut self here
            // so make a static struct to hold it i guess..  
        };
    }
}


pub fn setup(board: &mut impl RRIVBoard) {

    unsafe { MODBUS_BUFFER = Some(ModbusBuffer::<64>::new()
        .min_frame_len(3)
        .overwrite(true)) };

    let byte_processor = Box::<ModbusByteProcessor>::leak(Box::new(ModbusByteProcessor::new()));

    // pass a pointer to the leaked processor to Board::set_rx_processor
    board.set_serial_rx_processor(rriv_board::SerialRxPeripheral::SerialPeripheral2, Box::new(byte_processor));
}

// pub fn take_command(board: &impl RRIVBoard) -> Result<[u8; USART_BUFFER_SIZE], ()> {
//     // rprintln!("pending messages {}", pending_message_count(board));
//     if pending_message_count(board) < 1 {
//         return Err(());
//     }

//     let do_take_command = || unsafe {
//         let message_data = MESSAGE_DATA.borrow_mut();
//         Ok(_take_command(message_data))
//     };

//     board.critical_section(do_take_command)
// }
