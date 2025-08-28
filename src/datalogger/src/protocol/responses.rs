use alloc::boxed::Box;
use alloc::format;
use rriv_board::RRIVBoard;
use rtt_target::rprintln;
use serde_json::{json, Value};

use crate::{alloc::string::ToString, drivers::types::CalibrationPair};

pub fn send_command_response_message(board: &mut impl RRIVBoard, message: &str) {
    rprintln!("{}", message);
    board.usb_serial_send(json!({"message":message}).to_string().as_str());
    board.usb_serial_send("\n");
}

pub fn send_command_response_error(board: &mut impl RRIVBoard, message: &str, error: &str) {
    board.usb_serial_send(
        json!({"status":"error", "message":message, "error": error})
            .to_string()
            .as_str(),
    );
    board.usb_serial_send("\n");
}

pub fn send_json(board: &mut impl RRIVBoard, json: Value) {
    board.usb_serial_send(json.to_string().as_str());
    board.usb_serial_send("\n");
}

pub fn calibration_point_list(board: &mut impl RRIVBoard, pairs: &Option<Box<[CalibrationPair]>>) {
    board.usb_serial_send("{ pairs: [");

    if let Some(pairs) = pairs {
        for i in 0..pairs.len() {
            rprintln!("calib pair{:?}", i);
            let pair = &pairs[i];
            board.usb_serial_send(format!("{{'point': {}, 'values': [", pair.point).as_str());
            for i in 0..pair.values.len() {
                board.usb_serial_send(format!("{}", pair.values[i]).as_str());
                if i < pair.values.len() - 1 {
                    board.usb_serial_send(",");
                }
            }

            board.usb_serial_send("] }");
            if i < pairs.len() - 1 {
                board.usb_serial_send(",");
            }
        }
    }

    board.usb_serial_send("]}\n");
}

pub fn device_get(board: &mut impl RRIVBoard, mut serial_number: [u8;5], mut uid : [u8;12]){
    rprintln!("{:?}", serial_number);
    let serial_number = util::str_from_utf8(&mut serial_number).unwrap_or_default();
    let uid = format!("{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}",            
            uid[0],
            uid[1],
            uid[2],
            uid[3],
            uid[4],
            uid[5],
            uid[6],
            uid[7],
            uid[8],
            uid[9],
            uid[10],
            uid[11]);
    let uid = uid.as_str();
    let json = json!({
        "serial_number": serial_number,
        "uid": uid
    });
    send_json(board, json);
}