use crate::{drivers::types::SensorDriver, services::sdi12_service};
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
    data_received: [f32; 36],
    num_data: usize,
    index: usize,
    start: usize,
    mode: u8, // 0 for command mode, 1 for data mode
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
        board.set_gpio_pin_mode(5, rriv_board::gpio::GpioMode::PullDownInput);
        let mut sdi12_service = sdi12_service::Sdi12TxProcessor::new(self.special_config.gpio, self.special_config.sensor_address);
        sdi12_service.setup();
    }

    fn get_measured_parameter_count(&mut self) -> usize {
        self.num_data as usize
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        Ok(self.data_received[index] as f64)
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8; 16] {
        let mut buffer = [0u8; 16];
        buffer[0..6].copy_from_slice("value_".as_bytes());
        if index < 10 {
            let c = char::from_digit(index as u32, 10).unwrap();
            let a = c as u8;
            buffer[6] = a;
        } else {
            let c = char::from_digit((index as u32) % 10, 10).unwrap();
            let a = c as u8;
            buffer[7] = a;
            let c = char::from_digit((index as u32) / 10, 10).unwrap();
            let a = c as u8;
            buffer[6] = a;
        }
        buffer
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::RRIVBoard) {

        // clear the data
        self.data_received =  [0.0; 36];

        // let mut sdi12_service = sdi12_service::Sdi12TxProcessor::new(self.special_config.gpio, self.special_config.sensor_address);
        // loop {
        //     sdi12_service.send_break(board);
        //     match sdi12_service.send_m_command(board, '0') {
        //         Some(m_response) => {
        //             self.num_data = m_response.n as usize;
        //             defmt::println!("Response received:\nttt: {}\tn: {}", m_response.ttt, m_response.n);
        //             if m_response.ttt > 0 {
        //                 // process the delay
        //                 let mut now = board.epoch_timestamp();
        //                 let trigger = now + m_response.ttt as i64;
        //                 while now < trigger {
        //                     board.usb_serial_send(format_args!("SDI12: waiting...\n"));
        //                     board.run_loop_iteration(); // feeds the watchdog and keeps the board layer updated
        //                     board.delay_ms(1000);
        //                     now = board.epoch_timestamp();
        //                 }
        //                 break;
        //             }
        //         }
        //         None => {
        //             self.num_data = 0;
        //             defmt::println!("Invalid ack to M command. Retrying...");
        //             board.delay_ms(1000);
        //             board.run_loop_iteration();
        //         }
        //     }
        // }
        // let mut total_measurements: usize = 0;
        // loop {
        //     sdi12_service.send_break(board);
        //     match sdi12_service.send_ha_command(board) {
        //         Some(ha_response) => {
        //             self.num_data = ha_response.nnn as usize;
        //             total_measurements = ha_response.nnn as usize;
        //             defmt::println!("Response received:\nttt: {}\tn: {}", ha_response.ttt, ha_response.nnn);
        //             if ha_response.ttt > 0 {
        //                 // process the delay
        //                 let mut now = board.epoch_timestamp();
        //                 let trigger = now + ha_response.ttt as i64;
        //                 while now < trigger {
        //                     board.usb_serial_send(format_args!("SDI12: waiting...\n"));
        //                     board.run_loop_iteration(); // feeds the watchdog and keeps the board layer updated
        //                     board.delay_ms(1000);
        //                     now = board.epoch_timestamp();
        //                 }
        //                 break;
        //             }
        //         }
        //         None => {
        //             self.num_data = 0;
        //             defmt::println!("Invalid ack to M command. Retrying...");
        //             board.delay_ms(1000);
        //             board.run_loop_iteration();
        //         }
        //     }
        // }

        // let mut index: usize = 0;
        // let mut start = 0;
        // sdi12_service.send_break(board);    // Break for D0 only
        // loop {
        //     match sdi12_service.send_d_command(board, index as u8) {
        //         Some(d_response) => {
        //             let end = start + d_response.count as usize;
        //             for i in start..end {
        //                 self.data_received[i] = d_response.data[i-start];
        //             }

        //             if end == total_measurements as usize {
        //                 defmt::println!("Received all data!");
        //                 break;
        //             }
        //             start = end;
        //             if index == 999 {
        //                 defmt::println!("Sent D999 and still didn't receive all the data");
        //                 break;
        //             }
        //             else {
        //                 index += 1;
        //                 board.delay_ms(1000);
        //                 board.run_loop_iteration();
        //             }
        //         },
        //         None => {
        //             board.delay_ms(1000);
        //             board.run_loop_iteration();
        //             sdi12_service.send_break(board);
        //         }
        //     }
        // }

        // Interrupt Implementation
        let max  = 20;
        if self.mode == 0 {
            for i in 0..max {
                let ack = self.command_mode(board);
                if ack {
                    break;
                }
                if i == max - 1 {
                    defmt::println!("SDI12: command timed out");
                }
            }
        }
        if self.mode == 1 {
            for i in 0..max {
                let done = self.data_mode(board);
                if done {
                    break;
                }
                if i == max - 1 {
                    self.mode = 0;
                    defmt::println!("SDI12: data timed out");
                }
            }
        }
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
            data_received: [0.0; 36],
            num_data: 0,
            index: 0,
            start: 0,
            mode: 0,
        }
    }

    pub fn command_mode(&mut self, board: &mut dyn rriv_board::RRIVBoard) -> bool {
        let mut sdi12_service = sdi12_service::Sdi12TxProcessor::new(self.special_config.gpio, self.special_config.sensor_address);
        sdi12_service.send_break(board);
        sdi12_service.send_command(board, sdi12_service::Sdi12Command::HA);

        let mut ack_received = false;
        // if received, process the buffer
        let buffer = sdi12_service.read_response(board);
        if buffer.is_err() {
            self.num_data = 0;
            defmt::println!("Timeout to HA command. Retrying...");
            self.mode = 0; // stay in command mode
            return ack_received;
        }
        let ha_response = sdi12_service.parse_ha_command(buffer.unwrap());
        match ha_response {
            Some(ha_response) => {
                self.num_data = ha_response.nnn as usize;
                defmt::println!("Response received:\nttt: {}\tn: {}", ha_response.ttt, ha_response.nnn);
                if ha_response.ttt > 0 {
                    // process the delay
                    let mut now = board.epoch_timestamp();
                    let trigger = now + ha_response.ttt as i64;
                    while now < trigger {
                        board.usb_serial_send(format_args!("SDI12: waiting...\n"));
                        board.run_loop_iteration(); // feeds the watchdog and keeps the board layer updated
                        board.delay_ms(1000);
                        now = board.epoch_timestamp();
                    }
                }
                self.mode = 1; // switch to data mode
                self.index = 0;
                ack_received = true;
            },
            None => {
                self.num_data = 0;
                defmt::println!("Invalid ack to M command.");
                self.mode = 0; // stay in command mode
                ack_received = false;
            }
        }
        ack_received
    }

    pub fn data_mode(&mut self, board: &mut dyn rriv_board::RRIVBoard) -> bool {
        let mut sdi12_service = sdi12_service::Sdi12TxProcessor::new(self.special_config.gpio, self.special_config.sensor_address);
        // if self.index == 0 {
            sdi12_service.send_break(board);    // Break for D0 only
        // }
        let ind_char = (b'0' + (self.index as u8)) as char;
        sdi12_service.send_command(board, sdi12_service::Sdi12Command::D(ind_char));

        // if received, process the buffer
        let buffer = sdi12_service.read_response(board);
        if buffer.is_err() {
            defmt::println!("Timeout to D{} command. Retrying...", self.index);
            self.mode = 1; // stay in data mode to try again
            return false;
        }
        let mut received_all_data = false;
        let d_response = sdi12_service.parse_d_command(board, buffer.unwrap());
        match d_response {
            Some(d_response) => {
                let end = self.start + d_response.count as usize;
                for i in self.start..end {
                    self.data_received[i] = d_response.data[i-self.start];
                    defmt::println!("Received data[{}]: {}", i, self.data_received[i]);
                }

                if end >= self.num_data as usize {
                    defmt::println!("Received all data!");
                    self.mode = 0; // switch back to command mode for next measurement
                    self.index = 0;
                    self.start = 0;
                    received_all_data = true;
                }
                else if self.index == 999 {
                    defmt::println!("Sent D999 and still didn't receive all the data");
                    self.mode = 0; // switch back to command mode to try again next time
                    self.index = 0;
                    self.start = 0;
                }
                else {
                    self.start = end;
                    self.index += 1;
                }
            },
            None => {
                defmt::println!("Invalid ack to D{} command.", self.index);
                self.mode = 1; // stay in data mode to try again
            }
        }
        board.delay_ms(50);
        board.run_loop_iteration();
        received_all_data
    }

}