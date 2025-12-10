use control_interface::command_recognizer::{CommandData, CommandRecognizer};
use control_interface::command_registry::CommandType;
use rriv_board::{RRIVBoard, RXProcessor};

extern crate alloc;
use alloc::boxed::Box;
use alloc::format;
use serde_json::Value;

use core::borrow::BorrowMut;
use core::ffi::CStr;
use serde::{Deserialize, Serialize};

use rtt_target::rprintln;


use crate::datalogger::payloads::*;

static mut COMMAND_DATA: CommandData = CommandData::default();

// pub struct CommandService {
//     // registry: CommandRegistry,
// }

#[derive(Serialize, Deserialize)]
struct CLICommand<'a> {
    object: &'a str,
    action: &'a str,
    subcommand: Option<Box<str>>
}

// impl CommandService {
// pub fn new() -> Self {
//     // set the static, shareable command data
//     // CommandService {
//     //     registry: CommandRegistry::new(),
//     // }
// }

/// set the global rx processor
pub fn setup(board: &mut impl RRIVBoard) {
    let char_processor = Box::<CharacterProcessor>::leak(Box::new(CharacterProcessor::new()));

    // pass a pointer to the lleaked processor to Board::set_rx_processor
    board.set_serial_rx_processor( rriv_board::SerialRxPeripheral::CommandSerial, Box::new(char_processor));
}


fn pending_message_count(board: &impl RRIVBoard) -> usize {
    let get_pending_message_count = || unsafe {
        let command_data = COMMAND_DATA.borrow_mut();
        CommandRecognizer::pending_message_count(&command_data)
    };

    board.critical_section(get_pending_message_count)
}

fn take_command(board: &impl RRIVBoard) -> Result<[u8; 200], ()> {
    let do_take_command = || unsafe {
        let command_data = COMMAND_DATA.borrow_mut();
        Ok(CommandRecognizer::take_command(command_data))
    };

    board.critical_section(do_take_command)
}

pub fn get_pending_command(board: &impl RRIVBoard) -> Option<Result<CommandPayload, CommandError>> {
    if pending_message_count(board) > 0 {
        if let Ok(command_bytes) = take_command(board) {
            let command_cstr = CStr::from_bytes_until_nul(&command_bytes).unwrap(); // TODO: do we really need CStr here?  I don't think so...
            let command_identification_result = identify_serial_command(command_cstr);
            // rprintln!("{:?}", command_identification_result);
            match command_identification_result {
                Ok(command_type) => {
                    let result: Result<CommandPayload, _> =
                        get_command_payload(command_type, command_cstr);
                    // rprintln!("{:?}", result);
                    return Some(result);
                }
                Err(error) => {
                    rprintln!("{:?} {:?}", error, command_cstr);
                    return Some(Err(error))
                }
            }
        }
    }
    return None;
}

pub fn get_command_from_parts(object: &str, action: &str, subcommmand: Option<Box<str>>) -> CommandType {
    if let Some(subcommand) = subcommmand {
        let command_str = format!("{}_{}_{}", object, action, subcommand);
        rprintln!("{:?}", command_str);
        CommandType::from_str(&command_str)
    } else {
        let command_str = format!("{}_{}", object, action);
        rprintln!("{:?}", command_str);
        CommandType::from_str(&command_str)
    }
}

fn identify_serial_command(command_cstr: &CStr) -> Result<CommandType, CommandError> {
    match parse_command::<CLICommand>(command_cstr) {
        Ok(cli_command) => {
            let command_type = get_command_from_parts(cli_command.object, cli_command.action, cli_command.subcommand);
            if command_type == CommandType::Unknown {
                return Err(CommandError::InvalidCommand);
            }
            return Result::Ok(command_type);
        }
        Err(e) => {
            return Result::Err(CommandError::ParseError(e));
        }
    }
}

fn parse_command<'a, T>(command_cstr: &'a CStr) -> Result<T, serde_json::Error>
//Option<T> // use Result with custom error type instead
where
    T: Deserialize<'a>,
{
    let command_data_str = command_cstr.to_str().unwrap();
    rprintln!("{}", command_data_str);

    return serde_json::from_str::<T>(command_data_str);
}

#[macro_export]
macro_rules!  parse_command_to_payload {
    ($payload_type:ty, $variant:path, $command_cstr:expr) => {
        let result = parse_command::<$payload_type>($command_cstr);
        match result {
            Ok(payload) => return Ok($variant(payload)),
            Err(error) => return Err(CommandError::InvalidPayload(error)),
        }
    };
}

fn get_command_payload(
    command: CommandType,
    command_cstr: &CStr,
) -> Result<CommandPayload, CommandError> {
    match command {
        CommandType::DataloggerSet => {
                        parse_command_to_payload!(DataloggerSetPayload, CommandPayload::DataloggerSet, command_cstr);
            }
        CommandType::DataloggerGet => {
                parse_command_to_payload!(DataloggerGetPayload, CommandPayload::DataloggerGet, command_cstr);
            }
        CommandType::DataloggerReset => todo!(),
        CommandType::DataloggerSetMode => {
                parse_command_to_payload!(DataloggerSetModeCommandPayload, CommandPayload::DataloggerSetModeCommandPayload, command_cstr);
            },
        CommandType::SensorSet => {

                let command_data_str = command_cstr.to_str().unwrap();

                let raw_value: Value = serde_json::from_str(command_data_str).unwrap(); // use hashbrown HashMap?

                let result = parse_command::<SensorSetPayload>(command_cstr);
                match result {
                    Ok(payload) => return Ok(CommandPayload::SensorSet(payload, raw_value)),
                    Err(error) => return Err(CommandError::InvalidPayload(error)),
                }

            },
        CommandType::SensorGet => {
                parse_command_to_payload!(SensorGetPayload, CommandPayload::SensorGet, command_cstr);
            },
        CommandType::SensorRemove => {
                parse_command_to_payload!(SensorRemovePayload, CommandPayload::SensorRemove, command_cstr);
            },
        CommandType::SensorList => {
                parse_command_to_payload!(SensorListPayload, CommandPayload::SensorList, command_cstr);
            },
        CommandType::SensorCalibratePoint => {
                // rprintln!("parsing SensorCalibratePoint");
                parse_command_to_payload!(SensorCalibratePointPayload, CommandPayload::SensorCalibratePoint, command_cstr);
            },
        CommandType::SensorCalibrateList => {
                parse_command_to_payload!(SensorCalibrateListPayload, CommandPayload::SensorCalibrateList, command_cstr);
            }
        CommandType::SensorCalibrateRemove => {
                parse_command_to_payload!(SensorCalibrateRemovePayload, CommandPayload::SensorCalibrateRemove, command_cstr);
            }
        CommandType::SensorCalibrateFit => {
                parse_command_to_payload!(SensorCalibrateFitPayload, CommandPayload::SensorCalibrateFit, command_cstr);
            },
        CommandType::SensorCalibrateClear => {
                parse_command_to_payload!(SensorCalibrateClearPayload, CommandPayload::SensorCalibrateClear, command_cstr);
            },
        CommandType::SensorReset => todo!(),
        CommandType::ActuatorSet => todo!(),
        CommandType::ActuatorGet => todo!(),
        CommandType::ActuatorRemove => todo!(),
        CommandType::ActuatorList => todo!(),
        CommandType::ActuatorReset => todo!(),
        CommandType::TelemeterSet => todo!(),
        CommandType::TelemeterGet => {
                Ok(CommandPayload::TelemeterGet)
            }
        CommandType::TelemeterRemove => todo!(),
        CommandType::TelemeterList => todo!(),
        CommandType::TelemeterReset => todo!(),
        CommandType::BoardVersion => todo!(),
        CommandType::BoardFirmwareWarranty => todo!(),
        CommandType::BoardFirmwareConditions => todo!(),
        CommandType::BoardFirmwareLicense => todo!(),
        CommandType::BoardRtcSet => {
                parse_command_to_payload!(BoardRtcSetPayload, CommandPayload::BoardRtcSet, command_cstr);
            },
        CommandType::BoardGet => {
                parse_command_to_payload!(BoardGetPayload, CommandPayload::BoardGet, command_cstr);
            }
        CommandType::BoardRestart => todo!(),
        CommandType::BoardI2cList => todo!(),
        CommandType::BoardMemoryCheck => todo!(),
        CommandType::BoardMcuStop => todo!(),
        CommandType::BoardMcuSleep => todo!(),
        CommandType::BoardSignalExAdcHigh => todo!(),
        CommandType::BoardSignalExAdcLow => todo!(),
        CommandType::BoardSignal3v3BoostHigh => todo!(),
        CommandType::BoardSignal3v3BoostLow => todo!(),
        CommandType::BoardSerialSend => {
                parse_command_to_payload!(BoardSerialSendPayload, CommandPayload::BoardSerialSend, command_cstr);
            }
        CommandType::DeviceSetSerialNumber => {
                parse_command_to_payload!(DeviceSetSerialNumberPayload, CommandPayload::DeviceSetSerialNumber, command_cstr);
            }
        CommandType::DeviceGet => {
                parse_command_to_payload!(DeviceGetPayload, CommandPayload::DeviceGet, command_cstr);
            }
        CommandType::Unknown => todo!(),
    }
}

/// I believe the lifetimes here are generic over the lifetime of the CommandData
pub struct CharacterProcessor {}

impl<'a> CharacterProcessor {
    pub fn new() -> CharacterProcessor {
        CharacterProcessor {}
    }
}

impl<'a, 'b> RXProcessor for CharacterProcessor {
    fn process_byte(&mut self, character: u8) {
        unsafe {
            let command_data = COMMAND_DATA.borrow_mut();
            CommandRecognizer::process_character(command_data, character);
        }
    }
}
