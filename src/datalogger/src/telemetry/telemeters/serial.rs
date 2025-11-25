use core::time;

use rriv_board::RRIVBoard;

use alloc::format;

// pub struct Serial {
// }


// impl Serial {
//     pub fn run_loop_iteration(&mut self, board: &mut impl RRIVBoard) {
//         board.usart_send(format!("timestamp:{}", board.timestamp()).as_str());
//     }
// }

pub fn run_loop_iteration(board: &mut impl RRIVBoard) {
    let timestamp = board.timestamp();
    board.usart_send(format!("timestamp:{}", timestamp).as_str());
}