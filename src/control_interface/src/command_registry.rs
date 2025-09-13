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
    pub fn from_str(cmd_str: &str) -> Self {
        match cmd_str {
            "datalogger_set" => CommandType::DataloggerSet,
            "datalogger_get" => CommandType::DataloggerGet,
            "datalogger_reset" => CommandType::DataloggerReset,
            "datalogger_set_mode" => CommandType::DataloggerSetMode,
            "sensor_set" => CommandType::SensorSet,
            "sensor_get" => CommandType::SensorGet,
            "sensor_remove" => CommandType::SensorRemove,
            "sensor_list" => CommandType::SensorList,
            "sensor_calibrate_point" => CommandType::SensorCalibratePoint,
            "sensor_calibrate_list" => CommandType::SensorCalibrateList,
            "sensor_calibrate_remove" => CommandType::SensorCalibrateRemove,
            "sensor_calibrate_fit" => CommandType::SensorCalibrateFit,
            "sensor_calibrate_clear" => CommandType::SensorCalibrateClear,
            "sensor_reset" => CommandType::SensorReset,
            "actuator_set" => CommandType::ActuatorSet,
            "actuator_get" => CommandType::ActuatorGet,
            "actuator_remove" => CommandType::ActuatorRemove,
            "actuator_list" => CommandType::ActuatorList,
            "actuator_reset" => CommandType::ActuatorReset,
            "telemeter_set" => CommandType::TelemeterSet,
            "telemeter_get" => CommandType::TelemeterGet,
            "telemeter_remove" => CommandType::TelemeterRemove,
            "telemeter_list" => CommandType::TelemeterList,
            "telemeter_reset" => CommandType::TelemeterReset,
            "board_version" => CommandType::BoardVersion,
            "board_firmware_warranty" => CommandType::BoardFirmwareWarranty,
            "board_firmware_conditions" => CommandType::BoardFirmwareConditions,
            "board_firmware_license" => CommandType::BoardFirmwareLicense,
            "board_set" => CommandType::BoardRtcSet,
            "board_get" => CommandType::BoardGet,
            "board_restart" => CommandType::BoardRestart,
            "board_i2c_list" => CommandType::BoardI2cList,
            "board_memory_check" => CommandType::BoardMemoryCheck,
            "board_mcu_stop" => CommandType::BoardMcuStop,
            "board_mcu_sleep" => CommandType::BoardMcuSleep,
            "board_signal_ex_adc_high" => CommandType::BoardSignalExAdcHigh,
            "board_signal_ex_adc_low" => CommandType::BoardSignalExAdcLow,
            "board_signal_3v3_boost_high" => CommandType::BoardSignal3v3BoostHigh,
            "board_signal_3v3_boost_low" => CommandType::BoardSignal3v3BoostLow,
            "serial_send" => CommandType::BoardSerialSend,
            "device_set" => CommandType::DeviceSetSerialNumber,
            "device_get" => CommandType::DeviceGet,
            _ => CommandType::Unknown,
        }
    }
}

type CommandMap = HashMap<CommandType, extern "C" fn(*const c_char)>;

pub struct CommandRegistry {
    command_map: CommandMap,
}

impl CommandRegistry {
    pub fn new() -> Self {
        // NOTE: This is a hack to get the number of commands. There is no better way.
        let num_commands = CommandType::Unknown as u8 as usize + 1;
        let command_map = HashMap::with_capacity(num_commands);
        CommandRegistry { command_map }
    }
    pub fn register_command(&mut self, command: CommandType, action_fn: extern "C" fn(*const c_char)) {
        self.command_map.insert(command, action_fn);
    }
    pub fn register_command_str(&mut self, command: &str, action_fn: extern "C" fn(*const c_char)) {
        let command = CommandType::from_str(command);
        self.register_command(command, action_fn);
    }
    // NOTE: This is not needed since get_action_fn returns an Option, but may be useful for testing.
    pub fn is_registered(&self, command: CommandType) -> bool {
        self.command_map.contains_key(&command)
    }
    pub fn get_action_fn(&self, command: &CommandType) -> Option<extern "C" fn(*const c_char)> {
        self.command_map.get(command).copied()
    }
   
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_command_registration() {
        let mut command_registry = CommandRegistry::new();
        let command = Command::DataloggerSet;
        extern "C" fn action_fn_1(_: *mut c_void) {}
        command_registry.register_command(command, action_fn_1);
        assert!(command_registry.is_registered(Command::DataloggerSet));
    }
    #[test]
    fn test_command_registration_str() {
        let mut command_registry = CommandRegistry::new();
        let cmd_str = "datalogger_set";
        extern "C" fn action_fn_2(_: *mut c_void) {}
        command_registry.register_command_str(cmd_str, action_fn_2);
        assert!(command_registry.is_registered(Command::DataloggerSet));
    }
    #[test]
    fn test_get_command_from_parts() {
        let command_registry = CommandRegistry::new();
        let command = Command::DataloggerSet;
        let object = "datalogger";
        let action = "set";
        // let subcommand = Box(str, "subcommand");
        let subcommand = None;
        let command_from_parts = command_registry.get_command_from_parts(object, action, subcommand);
        assert_eq!(command, command_from_parts);
    }
}
