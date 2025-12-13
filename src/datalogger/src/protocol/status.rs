use rriv_board::RRIVBoard;
use serde_json::json;

use crate::alloc::string::ToString;

pub fn send_ready_status(board: &mut impl RRIVBoard) {
    board.usb_serial_send(format_args!("{}\n", json!({"status":"datalogger-ready"}).to_string().as_str()));
}