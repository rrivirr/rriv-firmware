use alloc::boxed::Box;
use crate::drivers::{ types::{SensorDriver, SensorDriverGeneralConfiguration, SENSOR_SETTINGS_PARTITION_SIZE}};


const SENSOR_NAMES: [&str; 10] = [
    "no_match",
    "generic_analog",
    "atlas_ec",
    "aht20",
    "mcp_9808",
    "ring_temperature",
    "timed_switch_2",
    "ds18b20",
    "k30_co2",
    "ring_temp_sim",
];

pub fn sensor_type_id_from_name(name: &str) -> Result<u16, ()> {
    for i in 0..SENSOR_NAMES.len() {
        if name == SENSOR_NAMES[i] {
            return Ok(u16::try_from(i).ok().unwrap());
        }
    }
    Err(())
}

pub fn sensor_name_from_type_id(id: usize) -> [u8; 16] {
    let mut rval = [b'\0'; 16];
    let name = SENSOR_NAMES[id];
    rval[..name.len()].copy_from_slice(name.as_bytes());
    rval
}


/* start registry WIP */

#[macro_export]
macro_rules! driver_create_functions {
    ($driver:ty, $special_settings_type:ty) => {
        (
            |general_settings: SensorDriverGeneralConfiguration,
             special_settings_values: serde_json::Value|
             -> Result<Box<dyn SensorDriver>, &'static str> {
                
                let special_settings_result = <$special_settings_type>::parse_from_values(special_settings_values);
                let special_settings = match special_settings_result {
                    Ok(special_settings) => special_settings,
                    Err(message) => return Err(message)
                };
                
                let driver = <$driver>::new(general_settings, special_settings); // seems ok

                Ok(Box::new(driver))
            },
            |general_settings: SensorDriverGeneralConfiguration,
             special_settings_slice: &[u8]|
             -> Box<dyn SensorDriver> {
                let mut bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE] =
                    [0; SENSOR_SETTINGS_PARTITION_SIZE];
                bytes.clone_from_slice(special_settings_slice);
                let special_settings = <$special_settings_type>::new_from_bytes(bytes);
                let driver = <$driver>::new(general_settings, special_settings);
                Box::new(driver)
            },
        )
    };
}

type DriverCreateFunctions = Option<(
    fn(
        SensorDriverGeneralConfiguration,
        serde_json::Value,
    ) -> Result<Box<dyn SensorDriver>, &'static str>,
    fn(SensorDriverGeneralConfiguration, &[u8]) -> Box<dyn SensorDriver>,
)>;

pub fn get_registry() -> [DriverCreateFunctions; 256] {
    const ARRAY_INIT_VALUE: DriverCreateFunctions = None;
    let mut driver_create_functions: [DriverCreateFunctions; 256] = [ARRAY_INIT_VALUE; 256];
    driver_create_functions[0] = None;
    driver_create_functions[1] = Some(driver_create_functions!(
        crate::drivers::generic_analog::GenericAnalog,
        crate::drivers::generic_analog::GenericAnalogSpecialConfiguration
    )); 
    driver_create_functions[2] = Some(driver_create_functions!(
        crate::drivers::atlas_ec::AtlasEC,
        crate::drivers::atlas_ec::AtlasECSpecialConfiguration
    ));
    driver_create_functions[3] = Some(driver_create_functions!(
        crate::drivers::aht20::AHT20,
        crate::drivers::aht20::AHT20SpecialConfiguration
    ));
    driver_create_functions[4] = Some(driver_create_functions!(
        crate::drivers::mcp9808::MCP9808TemperatureDriver,
        crate::drivers::mcp9808::MCP9808TemperatureDriverSpecialConfiguration
    )); 
    driver_create_functions[5] = Some(driver_create_functions!(
        crate::drivers::ring_temperature::RingTemperatureDriver,
        crate::drivers::ring_temperature::RingTemperatureDriverSpecialConfiguration
    ));
    driver_create_functions[6] = Some(driver_create_functions!(
        crate::drivers::timed_switch_2::TimedSwitch2, 
        crate::drivers::timed_switch_2::TimedSwitch2SpecialConfiguration
    ));
    driver_create_functions[7] = Some(driver_create_functions!(
        crate::drivers::ds18b20::Ds18b20,
        crate::drivers::ds18b20::Ds18b20SpecialConfiguration
    ));
    driver_create_functions[8] = Some(driver_create_functions!(
        crate::drivers::k30_co2::K30CO2, 
        crate::drivers::k30_co2::K30CO2SpecialConfiguration
    ));
    driver_create_functions[9] = Some(driver_create_functions!(
        crate::drivers::ring_temperature_sim::RingTemperatureDriver,
        crate::drivers::ring_temperature_sim::RingTemperatureDriverSpecialConfiguration
    ));

    driver_create_functions
}

/* end registry WIP */