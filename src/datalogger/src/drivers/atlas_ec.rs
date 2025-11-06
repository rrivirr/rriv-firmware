use crate::sensor_name_from_type_id;

use crate::drivers::atlas::*;

use super::types::*;
use bitfield_struct::bitfield;
use alloc::boxed::Box;
use rtt_target::rprint;
use serde_json::json;



#[derive(Copy, Clone, Debug)]
pub struct AtlasECSpecialConfiguration {
    empty: [u8; 32]
}


pub struct AtlasEC {
    general_config: SensorDriverGeneralConfiguration,
    special_config: AtlasECSpecialConfiguration,
    measured_parameter_values: [f64; 2],
}


impl SensorDriver for AtlasEC {
    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
        rprint!("not implemented");
    }

    fn get_configuration_json(&mut self) -> serde_json::Value {
        rprint!("not implemented");
    }

    fn setup(&mut self) {
        rprint!("not implemented");
    }

    fn get_id(&self) -> [u8; 6] {
        rprint!("not implemented");
    }

    fn get_type_id(&mut self) -> u16 {
        rprint!("not implemented");
    }

    fn get_measured_parameter_count(&mut self) -> usize {
        todo!();
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        rprint!("not implemented");
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        todo!();
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        board.
        rprint!("not implemented");
    }

    fn update_actuators(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        rprint!("not implemented");
    }

    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
        todo!()
    }

    fn clear_calibration(&mut self) {
        rprint!("not implemented");
    }
}