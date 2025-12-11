extern crate alloc;
use core::ffi::c_char;
use hashbrown::HashMap;


/// NOTE: Since this has a C compatible representation, it could be used in the FFI
/// we use from_str when processing commands from the serial side anyway though..
#[repr(u8)]
#[derive(Eq, Hash, PartialEq)]
pub enum CommandType {
    DataloggerSet = 0,
    DataloggerGet = 1,
    DataloggerReset = 2,
    DataloggerSetMode = 3,
    SensorSet = 4,
    SensorGet = 5,
    SensorRemove = 6,
    SensorList = 7,
    SensorCalibratePoint = 8,
    SensorCalibrateList = 36,
    SensorCalibrateRemove = 37,
    SensorCalibrateFit = 9,
    SensorCalibrateClear = 38,
    SensorReset = 10,
    ActuatorSet = 11,
    ActuatorGet = 12,
    ActuatorRemove = 13,
    ActuatorList = 14,
    ActuatorReset = 15,
    TelemeterSet = 16,
    TelemeterGet = 17,
    TelemeterRemove = 18,
    TelemeterList = 19,
    TelemeterReset = 20,
    BoardVersion = 21,
    BoardFirmwareWarranty = 22,
    BoardFirmwareConditions = 23,
    BoardFirmwareLicense = 24,
    BoardRtcSet = 25,
    BoardGet = 26,
    BoardRestart = 27,
    BoardI2cList = 28,
    BoardMemoryCheck = 29,
    BoardMcuStop = 30,
    BoardMcuSleep = 31,
    BoardSignalExAdcHigh = 32,
    BoardSignalExAdcLow = 33,
    BoardSignal3v3BoostHigh = 34,
    BoardSerialSend = 39,
    BoardSignal3v3BoostLow = 35,
    DeviceSetSerialNumber = 40,
    DeviceGet = 41,
    Unknown = 42, // !!! `Unknown` needs to be the last command, its value is used to get the number of commands see CommandRegistry::new !!!
}

impl CommandType {
    // TODO: this is the right way to do it, without format!
    pub fn from(cmd: (&str, &str, &str)) -> Self {
        match cmd {
            ("datalogger", "set", "") => CommandType::DataloggerSet,
            ("datalogger", "get", "") => CommandType::DataloggerGet,
            ("datalogger","reset", "") => CommandType::DataloggerReset,
            ("datalogger","set","mode") => CommandType::DataloggerSetMode,
            ("sensor","set","") => CommandType::SensorSet,
            ("sensor","get", "") => CommandType::SensorGet,
            ("sensor","remove", "") => CommandType::SensorRemove,
            ("sensor","list", "") => CommandType::SensorList,
            ("sensor","calibrate","point") => CommandType::SensorCalibratePoint,
            ("sensor","calibrate","list") => CommandType::SensorCalibrateList,
            ("sensor","calibrate","remove") => CommandType::SensorCalibrateRemove,
            ("sensor","calibrate","fit") => CommandType::SensorCalibrateFit,
            ("sensor","calibrate","clear") => CommandType::SensorCalibrateClear,
            ("sensor","reset", "") => CommandType::SensorReset,
            ("actuator","set", "") => CommandType::ActuatorSet,
            ("actuator","get", "") => CommandType::ActuatorGet,
            ("actuator","remove", "") => CommandType::ActuatorRemove,
            ("actuator","list", "") => CommandType::ActuatorList,
            ("actuator","reset", "") => CommandType::ActuatorReset,
            ("telemeter","set", "") => CommandType::TelemeterSet,
            ("telemeter","get", "") => CommandType::TelemeterGet,
            ("telemeter","remove", "") => CommandType::TelemeterRemove,
            ("telemeter","list", "") => CommandType::TelemeterList,
            ("telemeter","reset", "") => CommandType::TelemeterReset,
            ("board","version", "") => CommandType::BoardVersion,
            ("board","firmware","warranty") => CommandType::BoardFirmwareWarranty,
            ("board","firmware","conditions") => CommandType::BoardFirmwareConditions,
            ("board","firmware","license") => CommandType::BoardFirmwareLicense,
            ("board","set", "") => CommandType::BoardRtcSet,
            ("board","get", "") => CommandType::BoardGet,
            ("board","restart", "") => CommandType::BoardRestart,
            ("board","i2c","list") => CommandType::BoardI2cList,
            ("board","memory","check") => CommandType::BoardMemoryCheck,
            ("board","mcu","stop") => CommandType::BoardMcuStop,
            ("board","mcu","sleep") => CommandType::BoardMcuSleep,
            ("board_signal_ex","adc","high") => CommandType::BoardSignalExAdcHigh,
            ("board_signal_ex","adc","low") => CommandType::BoardSignalExAdcLow,
            ("board_signal_3v3","boost","high") => CommandType::BoardSignal3v3BoostHigh,
            ("board_signal_3v3","boost","low") => CommandType::BoardSignal3v3BoostLow,
            ("serial","send", "") => CommandType::BoardSerialSend,
            ("device","set", "") => CommandType::DeviceSetSerialNumber,
            ("device","get", "") => CommandType::DeviceGet,
            _ => CommandType::Unknown,
        }
    }
}

