use rriv_board::gpio::GpioMode;
use rtt_target::rprintln;
use serde_json::json;
use alloc::string::*;
use alloc::format;

use crate::sensor_name_from_type_id;
use crate::services::usart_service;

use super::types::*;

pub fn checksum_calculator(_message: String) -> u32 {
    let message = _message.as_bytes();
    let mut sum: i32 = 0;
    for i in 0..message.len() {
        sum += message[i] as i32;
    }
    let crc: i32 = 255 - sum + 1;
    let crc =((crc % 256 + 256) % 256) as u32;
    crc
}

#[derive(Copy, Clone)]
pub struct BasicEvoSpecialConfiguration {
    device_address: usize,
    command: usize,
    start_address: usize,
    no_of_registers: usize,
    reg_address: usize,
    reg_value: usize,
}

impl BasicEvoSpecialConfiguration {

    pub fn parse_from_values(value: serde_json::Value) -> Result<BasicEvoSpecialConfiguration, &'static str> {
        // should we return a Result object here? because we are parsing?  parse_from_values?
        // Device Address
        let mut device_address: usize = 0x34;
        match &value["device_address"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            device_address = number;
                        }
                        Err(_) => return Err("invalid device address")
                    }
                }
            }
            _ => {
                return Err("device address is required");
            }
        }

        // Command
        let s = match &value["command"] {
            serde_json::Value::String(s) => s.as_str(),
            _ => return Err("command must be \"read\" or \"write\""),
        };
     
        let command: usize = match s.to_ascii_lowercase().as_str() {
            "read" => 0x03,
            "write" => 0x06,
            _ => return Err("invalid command"),
        };

        // Start Address
        let mut start_address: usize = 0x0080;
        match &value["start_address"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            start_address = number;
                        }
                        Err(_) => return Err("invalid start address")
                    }
                }
            }
            _ => {
                return Err("start address is required");
            }
        }

        // Number of Registers
        let mut no_of_registers: usize = 2;
        match &value["no_of_registers"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            no_of_registers = number;
                        }
                        Err(_) => return Err("invalid number of registers")
                    }
                }
            }
            _ => {
                return Err("number of registers is required");
            }
        }

        // Register Address
        let mut reg_address: usize = 0x0001;
        match &value["reg_address"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            reg_address = number;
                        }
                        Err(_) => return Err("invalid register address")
                    }
                }
            }
            _ => {
                return Err("register address is required");
            }
        }

        // Register Value
        let mut reg_value: usize = 0x0001;
        match &value["reg_value"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            reg_value = number;
                        }
                        Err(_) => return Err("invalid register value")
                    }
                }
            }
            _ => {
                return Err("register value is required");
            }
        }
      
        Ok ( Self {
            device_address,
            command,
            start_address,
            no_of_registers,
            reg_address,
            reg_value,
        } )
    }


    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> BasicEvoSpecialConfiguration {
        let settings = bytes.as_ptr().cast::<BasicEvoSpecialConfiguration>();
        unsafe { *settings }
    }
}

pub struct BasicEvo {
    general_config: SensorDriverGeneralConfiguration,
    special_config: BasicEvoSpecialConfiguration,
    message: String,
    register_values: [u16; 16],
}

impl BasicEvo {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: BasicEvoSpecialConfiguration,
    ) -> Self {
        BasicEvo {
            general_config,
            special_config,
            message: String::from(":340300800004AA"),
            register_values: [0; 16],
        }
    }
}

impl SensorDriver for BasicEvo {
    #[allow(unused)]
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        // Construct message
        let mut message  = String::from(":");
        message += &format!("{:02X}", self.special_config.device_address);
        message += &format!("{:02X}", self.special_config.command);
        if self.special_config.command == 0x6 {
            message += &format!("{:04X}", self.special_config.reg_address);
            message += &format!("{:04X}", self.special_config.reg_value);
        }
        else if self.special_config.command == 0x3 {
            message += &format!("{:04X}", self.special_config.start_address);
            message += &format!("{:04X}", self.special_config.no_of_registers);
        }

        let checksum = checksum_calculator(message.clone()[1..].to_string());
        message += &format!("{:02X}", checksum);
        self.message = message;
    }

    getters!();
    

    fn get_measured_parameter_count(&mut self) -> usize {
        self.special_config.no_of_registers
    }

    #[allow(unused)]
    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        Ok(self.register_values[index] as f64)
    }

    #[allow(unused)]
    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8;16] {
        return single_raw_or_cal_parameter_identifiers(index, Some(b'T'));
    }

    #[allow(unused)]
    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
    
        // UART Communication
        let prepared_message = format!("{}\r\n", self.message);
        let prepared_message = prepared_message.as_str();
        rprintln!("message {}", prepared_message);
        board.usart_send(prepared_message);
        board.delay_ms(500);      
    }

    fn receive_message(&mut self, message: Option<[u8; usart_service::USART_BUFFER_SIZE]>) {
        
        let response = match message {
            Some(message) => message,
            None => {
                rprintln!("no usart response");
                return;
            }
        };
        let nul_range_end = response.iter()
            .position(|&c| c == b'\0')
            .unwrap_or(response.len());
        let response = &response[..nul_range_end];
        let mut response_str = String::new();
        for b in response {
            response_str.push(*b as char);
        }
        rprintln!("response => {}", response_str);
        let trimmed_msg = response_str[1..].to_string();
        let trimmed_msg = trimmed_msg[..(trimmed_msg.len()-2)].to_string(); 
        rprintln!("trimmed message => {}, len: {}", trimmed_msg, trimmed_msg.len());

        let _trimmed_resp = &response[1..response.len()-2];

        let crc = checksum_calculator(trimmed_msg);
        let act_crc = u32::from_str_radix(&response_str[(response_str.len()-2)..], 16).unwrap_or(0);
        
        if crc != act_crc {
            rprintln!("checksum mismatch, crc: 0x{:02X}, act_crc: 0x{:02X}", crc, act_crc);
            return;
        }

        let response_str = &response_str[1..response_str.len()-2]; // remove : and checksum
        let device_address = &response_str[0..2];
        if usize::from_str_radix(device_address, 16).unwrap_or(0) != self.special_config.device_address{
            rprintln!("Device address mismatch!");
            return;
        }

        let _byte_count = &response_str[4..6];
        let data_bytes = &response_str[6..];

        for i in 0..self.special_config.no_of_registers {
            let val = &data_bytes[i*4..(i*4)+4];
            let register_value = u16::from_str_radix(val, 16).unwrap_or(0);
            self.register_values[i] = register_value;
            rprintln!("reg_values[{}] = {:04X}", i, register_value);
        }
    }

    fn get_requested_gpios(&self) -> super::resources::gpio::GpioRequest {
        let mut request = super::resources::gpio::GpioRequest::none();
        request.use_pin(6);
        request.use_usart();
        return request;
    }

    #[allow(unused)]
    fn update_actuators(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
    }
    
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

        let command_str: &str = match self.special_config.command {
            0x3 => "Read",
            0x5 => "Write",
            _ => "Unknown Command",
        };  

        if command_str == "Write" {
            return json!({
                "id" : sensor_id,
                "type" : sensor_name,
                "device_address": format!("0x{:X}", self.special_config.device_address),
                "command": command_str,
                "reg_address": format!("0x{:X}", self.special_config.reg_address),
                "reg_value": format!("{}", self.special_config.reg_value),        
            })
        } else if command_str == "Read" {
            return json!({
                "id" : sensor_id,
                "type" : sensor_name,
                "device_address": format!("0x{:X}", self.special_config.device_address),
                "command": command_str,
                "start_address": format!("0x{:X}", self.special_config.start_address),
                "no_of_registers": format!("{}", self.special_config.no_of_registers),
            })
        } else {
            return json!({
                "id" : sensor_id,
                "type" : sensor_name,
                "device_address": format!("0x{:X}", self.special_config.device_address),
                "command": command_str,      
            })
            
        }
    }
    
   
}
