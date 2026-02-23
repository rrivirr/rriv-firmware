use crate::drivers::types::SensorDriver;


struct MyDigitalPin {
    pub func new(&mut board);
    pub func release() -> &mut board;
}

impl digital_pin for MyDigitalPin {
    // the write and read functions
}

struct GroundwaterFlowSDI12 {

}


impl SensorDriver for GroundwaterFlowSDI12 {
    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
        todo!()
    }

    fn get_configuration_json(&mut self) -> serde_json::Value {
        todo!()
    }

    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        todo!()
    }

    fn get_id(&self) -> [u8; 6] {
        todo!()
    }

    fn get_type_id(&self) -> u16 {
        todo!()
    }

    fn get_measured_parameter_count(&mut self) -> usize {
        todo!()
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        todo!()
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        todo!()
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        let command = "aM"; // not done

        let my_digital_pin = MyDigitalPin::new(&mut board);


        sdi12::send_command(&commmand, my_digital_pin, my_delay_us);
    }
}