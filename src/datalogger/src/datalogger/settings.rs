use bitfield_struct::bitfield;
use rriv_board::EEPROM_DATALOGGER_SETTINGS_SIZE;
use util::{any_as_u8_slice, check_alphanumeric};

use crate::datalogger::payloads::DataloggerSettingsValues;


const DATALOGGER_SETTINGS_UNUSED_BYTES: usize = 13;
#[bitfield(u8)]
#[derive(PartialEq)]
pub struct DataloggerSettingsBitField {
    #[bits(1)]
    pub external_adc_enabled: bool,

    #[bits(1)]
    pub enable_lorawan_telemetry: bool,

    #[bits(1)]
    pub enable_modbus_rtu: bool,

    #[bits(1)]
    pub debug_includes_values: bool,

    #[bits(1)]
    pub withhold_incomplete_readings: bool,

    #[bits(1)]
    pub log_raw_data: bool,

    #[bits(1)]
    pub enable_interactive_logging: bool,

    #[bits(1)]
    unused: usize,
}



#[derive(Clone, Copy, PartialEq)]
pub struct DataloggerSettings {
    pub deployment_identifier: [u8; 16],
    pub logger_name: [u8; 8],
    pub site_name: [u8; 8],
    pub deployment_timestamp: u64,
    pub interactive_logging_interval: u16, //seconds
    pub sleep_interval: u16, // minutes
    pub start_up_delay: u16,
    pub delay_between_bursts: u16,
    pub bursts_per_measurement_cycle: u8,
    pub mode: u8,
    pub toggles: DataloggerSettingsBitField,
    reserved: [u8; DATALOGGER_SETTINGS_UNUSED_BYTES],
}

impl DataloggerSettings {
    pub fn new() -> Self {
        DataloggerSettings {
            deployment_identifier: [b'\0'; 16],
            logger_name: [b'\0'; 8],
            site_name: [b'\0'; 8],
            deployment_timestamp: 0,
            interactive_logging_interval: 1,
            sleep_interval: 15,
            bursts_per_measurement_cycle: 1,
            start_up_delay: 0,
            delay_between_bursts: 0,
            mode: b'i',
            toggles: DataloggerSettingsBitField::new(),
            reserved: [b'\0'; DATALOGGER_SETTINGS_UNUSED_BYTES],
        }
    }

    pub fn new_from_bytes(bytes: [u8; EEPROM_DATALOGGER_SETTINGS_SIZE]) -> DataloggerSettings {
        let settings = bytes.as_ptr().cast::<DataloggerSettings>();
        unsafe { *settings }
    }

    pub fn get_bytes(&mut self) -> [u8; EEPROM_DATALOGGER_SETTINGS_SIZE] {
        let mut bytes: [u8; EEPROM_DATALOGGER_SETTINGS_SIZE] =
            [0; EEPROM_DATALOGGER_SETTINGS_SIZE];
        let bytes_ref = unsafe { any_as_u8_slice(self) };
        bytes[0..bytes_ref.len()].clone_from_slice(bytes_ref);
        bytes
    }

    pub fn configure_defaults(self) -> DataloggerSettings {

        let mut settings = self.clone();

        if self.bursts_per_measurement_cycle == 0 || self.bursts_per_measurement_cycle > 20 {
            settings.bursts_per_measurement_cycle = 1;
        }

        if self.delay_between_bursts > 300_u16 {
            settings.delay_between_bursts = 0_u16
        }

        if self.interactive_logging_interval > 60_u16 || self.interactive_logging_interval < 1_u16 {
            settings.interactive_logging_interval = 1_u16;
        }

        if self.sleep_interval > 60_u16 * 4 {
            settings.sleep_interval = 15_u16;
        }


        if self.start_up_delay > 60_u16 {
            settings.start_up_delay = 0;
        }

        if !check_alphanumeric(&self.logger_name) {
            let default = "MyLogger";
            settings.logger_name[0..default.len()].clone_from_slice(default.as_bytes());
        }

        if !check_alphanumeric(&self.site_name) {
            let default = "site";
            settings.site_name[0..default.len()].clone_from_slice(default.as_bytes());
        }

        if !check_alphanumeric(&self.deployment_identifier) {
            let default = "deployment";
            settings.deployment_identifier[0..default.len()].clone_from_slice(default.as_bytes());
        }


        settings
    }

    pub fn with_values(self, values: DataloggerSettingsValues ) -> DataloggerSettings{
        let mut settings = DataloggerSettings::new();
        settings.deployment_identifier = values.deployment_identifier.unwrap_or(self.deployment_identifier);
        settings.logger_name = values.logger_name.unwrap_or(self.logger_name);
        settings.site_name = values.site_name.unwrap_or(self.site_name);
        settings.deployment_timestamp = values.deployment_timestamp.unwrap_or(self.deployment_timestamp);
        settings.interactive_logging_interval = values.interactive_logging_interval.unwrap_or(self.interactive_logging_interval);
        settings.sleep_interval = values.sleep_interval.unwrap_or(self.sleep_interval);
        settings.bursts_per_measurement_cycle = values.bursts_per_measurement_cycle.unwrap_or(self.bursts_per_measurement_cycle);
        settings.start_up_delay = values.start_up_delay.unwrap_or(self.start_up_delay);
        settings.delay_between_bursts = values.delay_between_bursts.unwrap_or(self.delay_between_bursts);
        settings.mode = values.mode.unwrap_or(self.mode);
        settings.toggles.set_enable_lorawan_telemetry(values.enable_lorawan_telemetry.unwrap_or(self.toggles.enable_lorawan_telemetry()));
        settings.toggles.set_enable_modbus_rtu(values.enable_modbus_rtu.unwrap_or(self.toggles.enable_modbus_rtu()));
        settings.toggles.set_enable_interactive_logging(values.interactive_logging.unwrap_or(self.toggles.enable_interactive_logging()));
        settings
    }

   
}


