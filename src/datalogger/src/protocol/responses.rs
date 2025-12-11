use alloc::boxed::Box;
use rriv_board::RRIVBoard;
use serde_json::{json, Value};

use crate::{alloc::string::ToString, drivers::types::CalibrationPair};

pub fn send_command_response_message(board: &mut impl RRIVBoard, message: &str) {
    defmt::println!("{}", message);
    board.usb_serial_send(format_args!("{}\n", json!({"message":message}).to_string().as_str()));
}

pub fn send_command_response_error(board: &mut impl RRIVBoard, message: &str, error: &str) {
    board.usb_serial_send(
        format_args!(
            "{}\n",
            json!({"status":"error", "message":message, "error": error})
                .to_string()
                .as_str(),
        )
    );
}

pub fn send_json(board: &mut impl RRIVBoard, json: Value) {
    board.usb_serial_send(format_args!("{}", json.to_string()));
    board.usb_serial_send(format_args!("\n"));
}

pub fn calibration_point_list(board: &mut impl RRIVBoard, pairs: &Option<Box<[CalibrationPair]>>) {
    board.usb_serial_send(format_args!("{{ pairs: ["));

    if let Some(pairs) = pairs {
        for i in 0..pairs.len() {
            defmt::println!("calib pair{:?}", i);
            let pair = &pairs[i];
            board.usb_serial_send(format_args!("{{'point': {}, 'values': [", pair.point));
            for i in 0..pair.values.len() {
                board.usb_serial_send(format_args!("{}", pair.values[i]));
                if i < pair.values.len() - 1 {
                    board.usb_serial_send(format_args!(","));
                }
            }

            board.usb_serial_send(format_args!("] }}"));
            if i < pairs.len() - 1 {
                board.usb_serial_send(format_args!(","));
            }
        }
    }

    board.usb_serial_send(format_args!("]}}\n"));
}

pub fn device_get(board: &mut impl RRIVBoard, mut serial_number: [u8;5], uid : [u8;12], mut gpio_assignments: [[u8;6];9]){
    defmt::println!("{:?}", serial_number);
    let serial_number = util::str_from_utf8(&mut serial_number).unwrap_or_default();
    let uid = format_args!("{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}",            
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
        "uid": uid,
        "gpio_bound" : {
            "gpio1" : util::str_from_utf8(&mut gpio_assignments[0]).unwrap_or_default(),
            "gpio2" : util::str_from_utf8(&mut gpio_assignments[1]).unwrap_or_default(),
            "gpio3" : "disabled",
            "gpio4" : "disabled",
            "gpio5" : util::str_from_utf8(&mut gpio_assignments[4]).unwrap_or_default(),
            "gpio6" : util::str_from_utf8(&mut gpio_assignments[5]).unwrap_or_default(),
            "gpio7" : util::str_from_utf8(&mut gpio_assignments[6]).unwrap_or_default(),
            "gpio8" : util::str_from_utf8(&mut gpio_assignments[7]).unwrap_or_default(),
            "usart" : util::str_from_utf8(&mut gpio_assignments[8]).unwrap_or_default(),
            // "usart_count" : gpio_assignments.usart_count(),
        }
    });
    send_json(board, json);
}