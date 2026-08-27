use alloc::boxed::Box;

use super::resources::gpio::*;


pub const SENSOR_SETTINGS_PARTITION_SIZE: usize = 32; // partitioning is part of the driver implemention, and not meaningful at the EEPROM level
pub type SensorGeneralSettingsSlice = [u8; SENSOR_SETTINGS_PARTITION_SIZE];
#[allow(dead_code)]
pub type SensorSpecialSettingsSlice = [u8; SENSOR_SETTINGS_PARTITION_SIZE];

#[derive(Copy, Clone)]
pub struct SensorDriverGeneralConfiguration {
    pub id: [u8; 6],
    pub sensor_type_id: u16,
    pub warmup: u16,
    pub readings_per_burst: u8,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct EmptySpecialConfiguration {}

impl SensorDriverGeneralConfiguration {
    pub fn new(id: [u8; 6], sensor_type_id: u16) -> SensorDriverGeneralConfiguration {
        Self {
            id: id,
            sensor_type_id: sensor_type_id,
            warmup: 0,
            readings_per_burst: 1,
        }
    }

    pub fn new_from_bytes(
        bytes: &SensorGeneralSettingsSlice,
    ) -> SensorDriverGeneralConfiguration {
        let settings = bytes.as_ptr().cast::<SensorDriverGeneralConfiguration>();
        unsafe { *settings }
    }

    pub fn empty() -> SensorDriverGeneralConfiguration {
        Self {
            id: [0u8; 6],
            sensor_type_id: 0,
            warmup: 0,
            readings_per_burst: 0,
       }
    }
}


pub struct CalibrationPair {
    pub point: f64,         // the reference value
    pub values: Box<[f64]>, // the raw values returned by the sensors
}

impl CalibrationPair {
    pub fn clone(&self) -> CalibrationPair{
        CalibrationPair {
            point: self.point,
            values: self.values.clone()
        }
    }
}



pub trait SensorDriver {

    // return a byte array representation of the driver configuration values
    // this gets stored in the EEPROM and loaded on boot by the datalogger
    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]); // derivable
    
    // return a json representatio nof the driver configuration values
    // this is used to communicate with rrivctl, the command line configuration tool, over USB
    fn get_configuration_json(&mut self) -> serde_json::Value;

    // update configuration values
    fn update(&mut self, values: serde_json::Value) -> Result<(),&'static str>;
    
    // implementation of logic necessary to set up the driver and sensor hardware for operations
    // this is called on sensor driver load, during configuration or boot
    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard);
    
    // return the identifier id of the driver, specificed by rrivctl
    // the getters!(); macro is used to implement this automatically in structs implementing SensorDriver
    fn get_id(&self) -> [u8; 6];
    
    // return the type id of the driver, a pre-assigned index specified by registry.rs
    // the getters!(); macro is used to implement this automatically in structs implementing SensorDriver
    fn get_type_id(&self) -> u16;

    // return the number of parameters this sensor measures
    fn get_measured_parameter_count(&mut self) -> usize;
    
    // return the values of the parameters measured by this sensor
    // index indicates which parameter value to return
    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()>;
    
    // return identifiers of the parameters measured by this sensor
    // identifiers are strings used to identify the parameters in logs and data files
    // index indicates which parameter identifiers to return
    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16];

    // get a measurement from the sensor
    // measurement is stored in implementation specific variables, and returned by get_measured_parameter_value
    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard);
    
    // perform any control actions implemented by the sensor
    // called before take_measurement in the datalogger's run loop
    // this function is optional
    #[allow(unused)]
    fn update_actuators(&mut self, board: &mut dyn rriv_board::RRIVBoard) {}

    // fit a calibration for the sensor based on stored calibration pairs
    // each calibration pair contains the raw sensor reading value, and a true value known to the operator
    // gathering and storage of calibration pairs is handled by the datalogger code
    // storage of fit parameters and application of the calibration to driver outputs must be implemented in the driver
    // implementing calibration, and this function. is optional
    #[allow(unused)]
    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> { 
        // error if fn called without a calibration routine implemented
        Err(()) 
    }

    // remove the current calibration from the sensor
    // implementing calibration, and this function. is optional
    fn clear_calibration(&mut self) {}

    // return GPIO pins needed by the driver
    // the datalogger uses this to avoid GPIO configuration confliects
    fn get_requested_gpios(&self) -> GpioRequest {
        GpioRequest::none()
    }


}


macro_rules! getters {

    () => {
        fn get_id(&self) -> [u8; 6] {
            self.general_config.id.clone()
        }

        fn get_type_id(&self) -> u16 {
            self.general_config.sensor_type_id.clone()
        }

        fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
            let generic_settings_bytes: &[u8] = unsafe { util::any_as_u8_slice(&self.general_config) };
            let special_settings_bytes: &[u8] = unsafe { util::any_as_u8_slice(&self.special_config) };

            copy_config_into_partition(0, generic_settings_bytes, storage);
            copy_config_into_partition(1, special_settings_bytes, storage);
        }
    };
}

pub(crate) use getters;
pub fn single_raw_or_cal_parameter_identifiers(index: usize, prefix: Option<u8>) -> [u8; 16] {
    let mut buf = [0u8; 16];
    let identifiers = ["raw", "cal"];
    let mut identifier = "invalid";
    if index <= 1 {
        identifier = identifiers[index];
    }
    let mut start = 0;
    if let Some(prefix) = prefix {
        start = 1;
        buf[0] = prefix;
    }
    for i in 0..identifier.len() {
        buf[i + start] = identifier.as_bytes()[i];
    }
    return buf;
}

pub fn copy_config_into_partition(
    partition: usize,
    bytes: &[u8],
    storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE],
) {
    let copy_size = if bytes.len() >= SENSOR_SETTINGS_PARTITION_SIZE {
        SENSOR_SETTINGS_PARTITION_SIZE
    } else {
        bytes.len()
    };
    let offset = SENSOR_SETTINGS_PARTITION_SIZE * partition;
    storage[offset..offset + copy_size].copy_from_slice(&bytes[0..copy_size]);
}