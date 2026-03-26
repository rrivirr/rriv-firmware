use crate::{drivers::types::SensorDriver, services::sdi12_service};
use sdi12::SDI12;
use crate::sensor_name_from_type_id;
use serde_json::json;
use super::types::*;

#[derive(Copy, Clone)]
pub struct GroundwaterFlowSDI12SpecialConfiguration {
    gpio: u8,
    sensor_address: char,
    measured_parameter_count: usize
}

impl GroundwaterFlowSDI12SpecialConfiguration {
    pub fn parse_from_values(value: serde_json::Value) -> Result<GroundwaterFlowSDI12SpecialConfiguration, &'static str> {
        
        let gpio_pin = match &value["gpio_pin"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    if number >= 1 && number <= 8 {
                        Some(number as u8)
                    } else {
                        return Err("invalid pin");
                    }           
                } else {
                    return Err("invalid pin");
                }
            }
            _ => {
                return Err("gpio pin is required")
            }
        };

        let address = match &value["sensor_address"] {
            serde_json::Value::String(s) => s.chars().next().unwrap_or('\0'),
            _ => return Err("sensor_address must be a single character from '0' to '9'"),
        };

        let measured_parameter_count = match &value["measured_parameter_count"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    if number <= 9 {
                        number as usize
                    } else {
                        return Err("measured_parameter_count must be between 0 and 9");
                    }
                } else {
                    return Err("invalid measured_parameter_count");
                }
            }
            _ => return Err("measured_parameter_count is required"),
        };

        Ok ( Self {
            gpio: gpio_pin.unwrap(),
            sensor_address: address,
            measured_parameter_count: measured_parameter_count
        } ) 
    }

    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> GroundwaterFlowSDI12SpecialConfiguration {
        let settings = bytes
            .as_ptr()
            .cast::<GroundwaterFlowSDI12SpecialConfiguration>();
        unsafe { *settings }
    }
}

pub struct GroundwaterFlowSDI12 {
    general_config: SensorDriverGeneralConfiguration,
    special_config: GroundwaterFlowSDI12SpecialConfiguration,
    data_received: [f32; 9],
    num_data: u8
}


impl SensorDriver for GroundwaterFlowSDI12 {

    getters!();

    fn get_configuration_json(&mut self) -> serde_json::Value {
        let mut sensor_id = self.get_id();
        let sensor_id = match util::str_from_utf8(&mut sensor_id) {
            Ok(sensor_id) => sensor_id,
            Err(_) => "Invalid",
        };

        let mut sensor_name = sensor_name_from_type_id(self.get_type_id().into());
        let sensor_name = match util::str_from_utf8(&mut sensor_name) {
            Ok(sensor_name) => sensor_name,
            Err(_) => "Invalid",
        };

        json!({
           "id" : sensor_id,
           "type" : sensor_name,
           "gpio_pin": self.special_config.gpio,
           "sensor_address": self.special_config.sensor_address,
           "measured_parameter_count": self.special_config.measured_parameter_count
        })
    }

    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {
        
    }

    fn get_measured_parameter_count(&mut self) -> usize {
        self.special_config.measured_parameter_count
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        Ok(self.data_received[index] as f64)
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let mut buffer = [0u8; 16];
        buffer[0..6].copy_from_slice("value_".as_bytes());
        let c = char::from_digit(index as u32, 10).unwrap();
        let a = c as u8;
        buffer[6] = a;
        buffer
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {

        let mut sdi12_service = sdi12_service::Sdi12ByteProcessor::new(self.special_config.gpio);
        let m_response = sdi12_service.send_m_command(board, self.special_config.sensor_address, '0');
        if m_response.address == '\0' {
            // invalid response
            defmt::println!("TIMEOUT error");
            return;
        }
        defmt::println!("Response received:\nttt: {}\tn: {}", m_response.ttt, m_response.n);
        if m_response.ttt > 0 {
            // process the delay
            let mut now = board.epoch_timestamp();
            let trigger = now + m_response.ttt as i64;
            while now < trigger {
                board.usb_serial_send(format_args!("SDI12: waiting...\n"));
                board.run_loop_iteration(); // feeds the watchdog and keeps the board layer updated
                board.delay_ms(1000);
                now = board.epoch_timestamp();
            }
        }

        let d_response = sdi12_service.send_d0_command(board, m_response.address, m_response.n);
        self.data_received = d_response.data;
        self.num_data = d_response.count;
        defmt::println!("Received data: {} {}", d_response.data[0], d_response.data[1]);
    }
}

impl GroundwaterFlowSDI12 {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: GroundwaterFlowSDI12SpecialConfiguration,
    ) -> Self {
        GroundwaterFlowSDI12 {
            general_config,
            special_config,
            data_received: [0.0; 9],
            num_data: 0
        }
    }
}