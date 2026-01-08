use alloc::fmt::Debug;
use rtt_target::rprint;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

const LOGGER_NAME_LENGTH: usize = 8;
const SITE_NAME_LENGTH: usize = 8;
const DEPLOYMENT_IDENTIFIER_LENGTH: usize = 16;

#[derive(Default)]
pub struct DataloggerSettingsValues {
    pub deployment_identifier: Option<[u8; 16]>,
    pub logger_name: Option<[u8; 8]>,
    pub site_name: Option<[u8; 8]>,
    pub deployment_timestamp: Option<u64>,
    pub interactive_logging_interval: Option<u16>,
    pub sleep_interval: Option<u16>,
    pub start_up_delay: Option<u16>,
    pub delay_between_bursts: Option<u16>,
    pub bursts_per_measurement_cycle: Option<u8>,
    pub mode: Option<u8>,
    pub enable_telemetry: Option<bool>
}

#[derive(Serialize, Deserialize)]
pub struct DataloggerSetPayload {
    pub object: Value,
    pub action: Value,
    pub logger_name: Option<Value>,
    pub site_name: Option<Value>,
    pub deployment_identifier: Option<Value>,
    pub interactive_logging_interval: Option<u16>,
    pub sleep_interval: Option<u16>,
    pub bursts_per_cycle: Option<u8>,
    pub start_up_delay: Option<u16>,
    pub mode: Option<Value>,
    pub enable_telemetry: Option<bool>
    // pub user_note: Option<Value>, // not implemented for now
    // pub user_value: Option<i16>
}

impl DataloggerSetPayload {
    pub fn values(self) -> DataloggerSettingsValues {
        let mut datalogger_settings_values = DataloggerSettingsValues::default();

        if let Some(value) = self.logger_name {
            match value {
                serde_json::Value::String(value) => {
                    let value = value.as_bytes();
                    let mut target = [0u8; LOGGER_NAME_LENGTH];
                    let len = value_length(&target, value);
                    target[0..len].clone_from_slice(&value[0..len]);
                    datalogger_settings_values.logger_name = Some(target);
                }
                _ => {}
            }
        }

        if let Some(value) = self.site_name {
            match value {
                serde_json::Value::String(value) => {
                    let value = value.as_bytes();
                    let mut target = [0u8; SITE_NAME_LENGTH];
                    let len = value_length(&target, value);
                    target[0..len].clone_from_slice(&value[0..len]);
                    datalogger_settings_values.site_name = Some(target);
                }
                _ => {}
            }
        }

        if let Some(value) = self.deployment_identifier {
            match value {
                serde_json::Value::String(value) => {
                    let value = value.as_bytes();
                    let mut target = [0u8; DEPLOYMENT_IDENTIFIER_LENGTH];
                    let len = value_length(&target, value);
                    target[0..len].clone_from_slice(&value[0..len]);
                    datalogger_settings_values.deployment_identifier = Some(target);
                }
                _ => {}
            }
        }

        if let Some(interactive_logging_interval) = self.interactive_logging_interval {
            datalogger_settings_values.interactive_logging_interval =
                Some(interactive_logging_interval);
        }

        if let Some(sleep_interval) = self.sleep_interval {
            datalogger_settings_values.sleep_interval = Some(sleep_interval);
        }

        if let Some(bursts_per_cycle) = self.bursts_per_cycle {
            datalogger_settings_values.bursts_per_measurement_cycle = Some(bursts_per_cycle);
        }

        if let Some(start_up_delay) = self.start_up_delay {
            datalogger_settings_values.start_up_delay = Some(start_up_delay);
        }

        if let Some(enable_telemetry) = self.enable_telemetry {
            datalogger_settings_values.enable_telemetry = Some(enable_telemetry);
        }

        datalogger_settings_values
    }
}

#[derive(Serialize, Deserialize)]
pub struct DataloggerGetPayload {
    pub object: Value,
    pub action: Value,
    pub propery: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct DataloggerSetModeCommandPayload {
    pub object: Value,
    pub action: Value,
    pub mode: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct SensorSetPayload {
    pub object: Value,
    pub action: Value,
    pub id: Option<Value>, // option
    pub r#type: Value,     
}

pub struct SensorSetPayloadValues {
    pub sensor_type_id: u16,
    pub sensor_id: Option<[u8; 6]>,
}

impl SensorSetPayload {
    pub fn  convert(&self) -> Result<SensorSetPayloadValues, &'static str> {
        let sensor_type_id = match &self.r#type {
            serde_json::Value::String(sensor_type) => {
                match crate::registry::sensor_type_id_from_name(&sensor_type) {
                    Ok(sensor_type_id) => sensor_type_id,
                    Err(_) => {
                        // responses::send_command_response_message(board, "sensor type not found");
                        return Err("sensor type not found");
                    }
                }
            }
            _ => {
                // responses::send_command_response_message(board, "sensor type not specified");
                return Err("sensor type not specified");
            }
        };

        let mut sensor_id = None;
        if let Some(payload_id) = &self.id {
            match payload_id {
                serde_json::Value::String(id) => {
                    let mut prepared_id: [u8; 6] = [0; 6];
                    if id.len() > 6 {
                        rprint!("WARNING: Sensor id too long, truncating to 6 characters.\n");
                    }
                    prepared_id[0..id.len().min(6)].copy_from_slice(&id.as_bytes()[0..id.len().min(6)]);
                    sensor_id = Some(prepared_id);
                }
                _ => {
                    // a unique ID will be created later
                }
            };
        }
        
        let values = SensorSetPayloadValues {
            sensor_id: sensor_id,
            sensor_type_id: sensor_type_id
        };
        Ok(values)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SensorGetPayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
}

#[derive(Serialize, Deserialize)]
pub struct SensorRemovePayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
}

pub struct SensorRemovePayloadValues {
    pub id: [u8;6]
}

impl SensorRemovePayload {
    pub fn convert(&self) -> Result<SensorRemovePayloadValues, &'static str> {
        match &self.id {
        serde_json::Value::String(id) => {
            let mut prepared_id: [u8; 6] = [0; 6];
            let len = id.as_bytes().len();
            let len = if len <= 6 { len } else { 6 };
            prepared_id[0..len].copy_from_slice(id.as_bytes());
            return Ok(SensorRemovePayloadValues {
                id: prepared_id
            })
        }
        _ => {
            return Err("Sensor id missing")
        }
    };
    }
}

#[derive(Serialize, Deserialize)]
pub struct SensorListPayload {
    pub object: Value,
    pub action: Value,
}

#[derive(Serialize, Deserialize)]
pub struct BoardRtcSetPayload {
    pub object: Value,
    pub action: Value,
    pub epoch: Value,
}

#[derive(Serialize, Deserialize)]
pub struct BoardGetPayload {
    pub object: Value,
    pub action: Value,
    pub parameter: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct BoardSerialSendPayload {
    pub object: Value,
    pub action: Value,
    pub message: Value,
}

pub struct BoardSerialSendCommandPayload {
    pub message: [u8; 20],
    pub message_len: u8,
}

impl BoardSerialSendPayload {
    pub fn convert(&self) -> Result<BoardSerialSendCommandPayload, &'static str> {
        // TODO: this static lifetime is not good?  Need to pass in some storage or use a box?
        let message = match self.message {
            serde_json::Value::String(ref message) => message,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad message");
            }
        };

        let mut message_bytes = [0u8; 20];
        message_bytes[0..message.len()].clone_from_slice(&message.as_bytes()[0..message.len()]);

        Ok(BoardSerialSendCommandPayload {
            message: message_bytes,
            message_len: message.len() as u8,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeviceSetSerialNumberPayload {
    pub object: Value,
    pub action: Value,
    pub serial_number: Option<Value>,
}

pub struct DeviceSetSerialNumberPayloadValues {
    pub serial_number: [u8; 5],
}

impl DeviceSetSerialNumberPayload {
    pub fn convert(&self) -> Result<DeviceSetSerialNumberPayloadValues, &'static str> {
        let serial_number = match self.serial_number {
            Some(serde_json::Value::String(ref serial_number)) => serial_number,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad serial number");
            }
        };

        // let mut bytes:
        let bytes = serial_number.as_str().as_bytes();
        if bytes.len() != 5 {
            return Err("wrong length");
        }
        let mut serial_number_bytes: [u8; 5] = [0; 5];
        serial_number_bytes.clone_from_slice(bytes);

        return Ok(DeviceSetSerialNumberPayloadValues {
            serial_number: serial_number_bytes,
        });
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeviceGetPayload {
    pub object: Value,
    pub action: Value,
}

#[derive(Serialize, Deserialize)]
pub struct SensorCalibratePointPayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
    pub subcommand: Value,
    pub point: Number,
    pub tag: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct SensorCalibrateListPayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
    pub subcommand: Value,
}

#[derive(Serialize, Deserialize)]
pub struct SensorCalibrateRemovePayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
    pub subcommand: Value,
    pub tag: Value,
}

#[derive(Serialize, Deserialize)]
pub struct SensorCalibrateFitPayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
    pub subcommand: Value,
}

#[derive(Serialize, Deserialize)]
pub struct SensorCalibrateClearPayload {
    pub object: Value,
    pub action: Value,
    pub id: Value,
    pub subcommand: Value,
}

pub struct SensorCalibrateSubcommand<'a> {
    pub object: &'a str,
    pub action: &'a str,
    pub id: &'a str,
    pub subcommand: &'a str,
}

// TODO: these impls should be derived or ??

impl SensorCalibrateFitPayload {
    pub fn convert(&'_ self) -> Result<SensorCalibrateSubcommand<'_>, &'static str> {
        let id = match self.id {
            serde_json::Value::String(ref payload_id) => payload_id,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad sensor id");
            }
        };

        return Ok(SensorCalibrateSubcommand {
            object: "sensor", // TODO: what's the most ideal way to handle these?  no real reason to convert them again
            action: "calibrate",
            id,
            subcommand: "fit",
        });
    }
}

impl SensorCalibrateClearPayload {
    pub fn convert(&'_ self) -> Result<SensorCalibrateSubcommand<'_>, &'static str> {
        let id = match self.id {
            serde_json::Value::String(ref payload_id) => payload_id,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad sensor id");
            }
        };

        return Ok(SensorCalibrateSubcommand {
            object: "sensor", // TODO: what's the most ideal way to handle these?  no real reason to convert them again
            action: "calibrate",
            id,
            subcommand: "clear",
        });
    }
}

impl SensorCalibrateListPayload {
    pub fn convert(&'_ self) -> Result<SensorCalibrateSubcommand<'_>, &'static str> {
        let id = match self.id {
            serde_json::Value::String(ref payload_id) => payload_id,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad sensor id");
            }
        };

        return Ok(SensorCalibrateSubcommand {
            object: "sensor", // TODO: what's the most ideal way to handle these?  no real reason to convert them again
            action: "calibrate",
            id,
            subcommand: "list",
        });
    }
}

pub struct SensorCalibrateRemove<'a> {
    pub object: &'a str,
    pub action: &'a str,
    pub id: &'a str,
    pub subcommand: &'a str,
    pub tag: &'a str,
}

impl SensorCalibrateRemovePayload {
    pub fn convert(&'_ self) -> Result<SensorCalibrateRemove<'_>, &'static str> {
        let id = match self.id {
            serde_json::Value::String(ref payload_id) => payload_id,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad sensor id");
            }
        };

        let tag = match self.tag {
            serde_json::Value::String(ref tag) => tag,
            _ => {
                // board.usb_serial_send("bad sensor id\n");
                return Err("bad calibration point tag");
            }
        };

        return Ok(SensorCalibrateRemove {
            object: "sensor", // TODO: what's the most ideal way to handle these?  no real reason to convert them again
            action: "calibrate",
            id,
            subcommand: "remove",
            tag,
        });
    }
}

#[derive(Serialize, Deserialize)]
pub enum CommandPayload {
    DataloggerSet(DataloggerSetPayload),
    DataloggerGet(DataloggerGetPayload),
    DataloggerSetModeCommandPayload(DataloggerSetModeCommandPayload), // deprecated
    SensorSet(SensorSetPayload, Value), // Value here is for dynamically specific special properties of the driver
    SensorGet(SensorGetPayload),
    SensorRemove(SensorRemovePayload),
    SensorList(SensorListPayload),
    SensorCalibratePoint(SensorCalibratePointPayload),
    SensorCalibrateList(SensorCalibrateListPayload),
    SensorCalibrateRemove(SensorCalibrateRemovePayload),
    SensorCalibrateFit(SensorCalibrateFitPayload),
    SensorCalibrateClear(SensorCalibrateClearPayload),
    BoardRtcSet(BoardRtcSetPayload),
    BoardGet(BoardGetPayload),
    BoardSerialSend(BoardSerialSendPayload),
    TelemeterGet,
    DeviceSetSerialNumber(DeviceSetSerialNumberPayload),
    DeviceGet(DeviceGetPayload),
}

// errors to use in refactor
pub enum CommandError {
    ParseError(serde_json::Error), // can we store a string with the error string in the the enum class?
    InvalidCommand,
    InvalidPayload(serde_json::Error),
}

impl Debug for CommandError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ParseError(arg0) => f.debug_tuple("ParseError").field(arg0).finish(),
            Self::InvalidCommand => write!(f, "InvalidCommand"),
            Self::InvalidPayload(arg0) => f.debug_tuple("InvalidPayload").field(arg0).finish(),
        }
    }
}

// pub fn get_id(id: &serde_json::Value) -> Result<alloc::string::String,(&str)> {
//     match id {
//         serde_json::Value::String(ref payload_id) => {
//             return Ok(payload_id)
//         },
//         _ => {
//             // board.usb_serial_send("bad sensor id\n");
//             return Err("bad sensor id")
//         }
//     };

// }

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
