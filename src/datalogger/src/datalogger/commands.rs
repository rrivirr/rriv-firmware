use rriv_board::{RRIVBoard, EEPROM_TOTAL_SENSOR_SLOTS};
use rtt_target::rprintln;
use serde_json::{json, Value};
extern crate alloc;
use crate::alloc::string::ToString;
use crate::datalogger::payloads::{
    SensorCalibratePointPayload, SensorSetPayloadValues,
};
use crate::drivers::types::{SensorDriverGeneralConfiguration};
use alloc::boxed::Box;

use crate::protocol::responses;
use crate::registry::sensor_name_from_type_id;
use crate::{datalogger::payloads::BoardGetPayload, drivers::types::SensorDriver};

// Command implementations

// TODO: this is a command implementation, probably doesn't below in this file
pub fn get_board(board: &mut impl RRIVBoard, payload: BoardGetPayload) {
    // let mut board_response = BoardResponse::new();

    if let Some(param) = payload.parameter {
        match param {
            serde_json::Value::String(param) => {
                rprintln!("{:?}", param.as_str());
                match param.as_str() {
                    "epoch" => {
                        let epoch = board.epoch_timestamp();
                        board.usb_serial_send(format_args!("{:}\n", epoch));
                    }
                    "version" => {
                        let mut branch = "none";
                        if let Some(found_branch) = option_env!("GIT_BRANCH") {
                            branch = found_branch;
                        }

                        let mut gitref = "none";
                        if let Some(found_ref) = option_env!("GIT_REF") {
                            gitref = found_ref;
                        }

                        let mut firmware_version = "none";
                        if let Some(found_ver) = option_env!("FIRMWARE_VERSION") {
                            firmware_version = found_ver;
                        }

                        let response = json!({
                            "hv":"0.4.2",
                            "fv":firmware_version,
                            "br":branch,
                            "ref":gitref
                        });


                        responses::send_command_response_message(
                            board,
                            response.to_string().as_str(),
                        );
                    }
                    "eeprom" => {
                        board.dump_eeprom();
                    }
                    _ => {
                        responses::send_command_response_message(
                            board,
                            "Unsupported param in command",
                        );
                    }
                }
            }
            err => {
                responses::send_command_response_message(board, "Bad param in command");
                rprintln!("Bad epoch {:?}", err);
                return;
            }
        }
    } else {
        let epoch = board.epoch_timestamp();
        board.usb_serial_send(format_args!("{:}", epoch));
    }
}

// convert values
// build driver
// check gpios / check if we can install it
// get the unique id
// set driver
// save in eeprom

pub fn build_driver( 
    payload_values: &SensorSetPayloadValues,
    raw_payload_values: Value,
) -> Result<Box<dyn SensorDriver>, &'static str> {
   

    rprintln!("looking up funcs");
    let registry = crate::registry::get_registry();
    let create_function = registry[usize::from(payload_values.sensor_type_id)];

    if let Some(functions) = create_function {
        let sensor_id = match payload_values.sensor_id {
            Some(id) => id,
            None => [0;6],
        };
        let general_settings = SensorDriverGeneralConfiguration::new(sensor_id, payload_values.sensor_type_id);

        match functions.0(general_settings, raw_payload_values) {
            Err(message) => {
                // responses::send_command_response_error(board, message, "");
                return Err(message);
            }

            Ok(driver) => return Ok(driver)
        }
    }

    Err("build fn missing") // panic?

}

pub fn find_empty_slot(
        drivers: &mut [Option<Box<dyn SensorDriver>>; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS]
) -> Option<usize> {
    for i in 0..drivers.len() {
        if drivers[i].is_none() {
            return Some(i);
        }
    }

    return None;
}


// NO LONGER USED
// TODO: for instance, this method could return a Result that has either a message/error or the payload to send bad
// TODO: better, this method does the editing on the drivers array and either succeeds or returns an error
// TODO: then the caller devides what to send to the device (including any re-query)
// pub fn set_sensor(
//     board: &mut impl RRIVBoard,
//     drivers: &mut [Option<Box<dyn SensorDriver>>; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS],
//     payload: SensorSetPayload,
//     raw_payload_values: Value,
// ) {
   

//     // get the sensor id or make a unique new one
//     let mut sensor_id: [u8; 6] = [b'0'; 6]; // base default value
//     if let Some(payload_id) = payload.id {
//         sensor_id = match payload_id {
//             serde_json::Value::String(id) => {
//                 let mut prepared_id: [u8; 6] = [0; 6];
//                 prepared_id[0..id.len()].copy_from_slice(id.as_bytes());
//                 prepared_id
//             }
//             _ => {
//                 // make a unique id
//                     let mut sensor_id: [u8; 6] = [b'0'; 6]; // base default value

//                 make_unique_sensor_id(drivers, sensor_id)
//             }
//         };
//     }

//     // find the slot
//     let mut slot = usize::MAX;
//     let mut empty_slot = usize::MAX;
//     for i in 0..drivers.len() {
//         if let Some(driver) = &mut drivers[i] {
//             if sensor_id == driver.get_id() {
//                 slot = i;
//             }
//         } else {
//             if empty_slot == usize::MAX {
//                 empty_slot = i;
//             }
//         }
//     }

//     if slot == usize::MAX {
//         slot = empty_slot
//     };

//     rprintln!("looking up funcs");
//     let registry = crate::registry::get_registry();
//     let create_function = registry[usize::from(sensor_type_id)];

//     if let Some(functions) = create_function {
//         let general_settings = SensorDriverGeneralConfiguration::new(sensor_id, sensor_type_id);

//         match functions.0(general_settings, raw_payload_values) {
//             Err(message) => {
//                 responses::send_command_response_error(board, message, "");
//                 return;
//             }

//             Ok((mut driver, special_settings_bytes)) => {
//                 let requested_gpios = driver.get_requested_gpios();


//                 driver.setup(board.get_sensor_driver_services());
//                 driver.gen

//                 // check the driver's gpio needs
//                 self.assigned_gpios.check_conflict(requested_gpios);

//                 drivers[slot] = Some(driver);

//                 // get the generic settings as bytes
//                 let generic_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&general_settings) };
//                 let mut bytes_sized = bytes::empty_sensor_settings();
//                 let copy_size = if generic_settings_bytes.len() >= SENSOR_SETTINGS_PARTITION_SIZE {
//                     SENSOR_SETTINGS_PARTITION_SIZE
//                 } else {
//                     generic_settings_bytes.len()
//                 };
//                 bytes_sized[..copy_size].copy_from_slice(&generic_settings_bytes[0..copy_size]);

//                 // get the special settings as bytes
//                 let copy_size = if special_settings_bytes.len() >= SENSOR_SETTINGS_PARTITION_SIZE {
//                     SENSOR_SETTINGS_PARTITION_SIZE
//                 } else {
//                     special_settings_bytes.len()
//                 };
//                 bytes_sized
//                     [SENSOR_SETTINGS_PARTITION_SIZE..(SENSOR_SETTINGS_PARTITION_SIZE + copy_size)]
//                     .copy_from_slice(&special_settings_bytes[0..copy_size]);

//                 // just use this, don't need to use what was passed
//                 driver.get_configuration_bytes(storage);
//                 board.store_sensor_settings(slot.try_into().unwrap(), &bytes_sized);
//             }
//         }
//     }

//     if let Some(driver) = &mut drivers[slot] {
//         responses::send_json(board, driver.get_configuration_json());
//         return;
//     }
// }

pub fn make_unique_sensor_id(
    drivers: &mut [Option<Box<dyn SensorDriver>>; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS],
    default: [u8; 6],
) -> [u8; 6] {
    let mut sensor_id = default.clone();
    // get all the current ids
    let mut driver_ids: [[u8; 6]; EEPROM_TOTAL_SENSOR_SLOTS] = [[0; 6]; EEPROM_TOTAL_SENSOR_SLOTS];
    for i in 0..drivers.len() {
        if let Some(driver) = &mut drivers[i] {
            driver_ids[i] = driver.get_id();
        }
    }

    let mut unique = false;
    while unique == false {
        let mut i = 0;
        let mut changed = false;
        let sensor_id_scan = sensor_id.clone();
        while i < driver_ids.len() && changed == false {
            let mut same = true;
            for (_j, (u1, u2)) in driver_ids[i].iter().zip(sensor_id_scan.iter()).enumerate() {
                // if hey are equal..
                if u1 != u2 {
                    same = false;
                    break;
                }
            }
            if same {
                // we need to make a change
                sensor_id[sensor_id.len() - 1] = sensor_id[sensor_id.len() - 1] + 1;
                changed = true;
            }
            i = i + 1;
        }
        unique = !changed;
    }
    sensor_id
}


// TDOD: in this case it's not so clear what would be ideal
// the list needs to be built, and we need to send it as we build it because it's too big to pass in memory
// so here direct sending to serial from the called function seems to be the right pattern
// are get/list and set/remove to be handled differently?
// since get is re-used by set as the return value, that could make sense.
pub fn list_sensors(
    board: &mut impl RRIVBoard,
    drivers: &[Option<Box<dyn SensorDriver>>; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS],
) {
    board.usb_serial_send(format_args!("{{\"sensors\":["));
    let mut first = true;
    for i in 0..drivers.len() {
        // create json and output it
        if let Some(driver) = &drivers[i] {
            if first {
                first = false
            } else {
                board.usb_serial_send(format_args!(","));
            }

            let mut id_bytes = driver.get_id();
            let id_str = match util::str_from_utf8(&mut id_bytes) {
                Ok(str) => str,
                Err(_) => "error",
            };

            let type_id = driver.get_type_id();
            let mut sensor_name_bytes = sensor_name_from_type_id(type_id.into());
            let sensor_name_str = match util::str_from_utf8(&mut sensor_name_bytes) {
                Ok(str) => str,
                Err(_) => "error",
            };

            let json = json!({
                "id": id_str,
                "type": sensor_name_str
            });
            let string = json.to_string();
            let str = string.as_str();
            board.usb_serial_send(format_args!("{}", str));
        }
    }
    board.usb_serial_send(format_args!("]}}\n"));
}

fn value_length(target: &[u8], value: &[u8]) -> usize {
    if value.len() > target.len() {
        target.len()
    } else {
        value.len()
    }
}

// fn match_and_set(source: Option<Value>, target: &mut [u8]) -> Option<[u8]> {
//     if let Some(value) = source {
//             match value {
//                 serde_json::Value::String(value) => {
//                     let value = value.as_bytes();
//                     target.clone_from_slice(&value[0..value_length(target, value)]);
//                     // has to set here somehow
//                 },
//                 _ => {

//                 },
//             }
//     }
// }

// TODO: a potential ordering in datalogger.rs
// 1. commands::sensor_add_calibration_point_arguments
// 2. get a refernce to the driver and measure a point, and get the array values  datalogger::measure_one_driver
// 3. commands::update the calibration points for the driver, or calibration::store_point
// 4. send the error text or the success text back to the datalogger

// use crate::Value::String;
use alloc::string::String;
pub fn sensor_add_calibration_point_arguments<'a>(
    payload: &'a SensorCalibratePointPayload,
) -> Result<(&'a String, f64), &'static str> {
    rprintln!("Sensor calibrate point payload");

    let id = match payload.id {
        serde_json::Value::String(ref payload_id) => payload_id,
        _ => return Err("Invalid sensor id specified"),
    };

    let point = payload.point.as_f64();
    let point = match point {
        Some(point) => point,
        None => {
            return Err("Invalid calibration point specified");
        }
    };

    Ok((id, point))
}

// pub fn sensor_add_calibration_point(
//     payload: SensorCalibratePointPayload,
//     drivers: &mut [Option<Box<dyn SensorDriver>>; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS],
//     calibration_point_values: &mut [Option<Box<[CalibrationPair]>>; EEPROM_TOTAL_SENSOR_SLOTS],
// ) -> Result<(), &'static str> {
//     // we want to do the book keeping here for point payloads
//     // i guess we use a box again

//     if let Some(index) = self.get_driver_index_by_id(id) {
//         if let Some(driver) = &mut self.sensor_drivers[index] {
//             // read sensor values
//             driver.take_measurement(board.get_sensor_driver_services()); // TODO: ugg this is the problem...  should this happen in the datalogger?   yes...

//             let count = driver.get_measured_parameter_count() / 2; // TODO: get_measured_parameter_count, vs get_output_parameter_count
//             let mut values = Box::new([0_f64; 10]); // TODO: max of 10, should we make this dynamic?
//             for j in 0..count {
//                 rprintln!("{:?}", j);
//                 let value = match driver.get_measured_parameter_value(j * 2) {
//                     Ok(value) => value,
//                     Err(_) => {
//                         // board.usb_serial_send("missing parameter value\n");
//                         0_f64
//                     }
//                 };
//                 values[j] = value;
//             }

//             // store the point
//             // TODO: we are only storing one point for now
//             let calibration_pair = CalibrationPair {
//                 point: point,
//                 values: values,
//             };
//             if calibration_point_values[index].is_some() {
//                 let pairs: &Option<Box<[CalibrationPair]>> = &calibration_point_values[index];
//                 if let Some(pairs) = pairs {
//                     let arr = [calibration_pair, pairs[0].clone()];
//                     let arr_box = Box::new(arr);
//                     calibration_point_values[index] = Some(arr_box);
//                 }
//             } else {
//                 let arr = [calibration_pair];
//                 let arr_box = Box::new(arr);
//                 calibration_point_values[index] = Some(arr_box);
//             }
//             // responses::send_command_response_message(
//             //     board,
//             //     "Stored a single calibration point",
//             // );
//         }
//     } else {
//         return Err("Didn't find the sensor");
//     }

//     Ok(())
// }
