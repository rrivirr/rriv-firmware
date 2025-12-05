use core::time;

use rriv_board::gpio::{self, GpioMode};
use rtt_target::rprintln;
use serde_json::json;

use crate::{any_as_u8_slice, sensor_name_from_type_id};

use super::types::*;

const MAX_MILLIS: u32 = 7000;
#[derive(Copy, Clone)]
pub struct TimedSwitch2SpecialConfiguration {
    on_time_s: usize,
    off_time_s: usize,
    gpio_pin: u8,
    initial_state: bool, // 'on' 'off'
    // polarity // 'low_is_on', 'high_is_on'
    period: f32,
    ratio: f32,
    _empty: [u8; 14],
}

impl TimedSwitch2SpecialConfiguration {

    pub fn parse_from_values(value: serde_json::Value) -> Result<TimedSwitch2SpecialConfiguration, &'static str> {
        // should we return a Result object here? because we are parsing?  parse_from_values?
        let mut on_time_s: usize = 10;
        match &value["on_time_s"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            on_time_s = number;
                        }
                        Err(_) => return Err("invalid on time")
                    }
                }
            }
            _ => {
                return Err("on time is required");
            }
        }

        let mut off_time_s: usize = 10;
        match &value["off_time_s"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    let number: Result<usize, _> = number.try_into();
                    match number {
                        Ok(number) => {
                            off_time_s = number;
                        }
                        Err(_) => return Err("invalid off time")
                    }
                }
            }
            _ => {
                return Err("off time is required")
            }
        }

        let mut gpio_pin : Option<u8> = None;
        match &value["gpio_pin"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_u64() {
                    if number >= 1 && number <= 8 { //TODO: this is annoying to have to code into each driver
                        gpio_pin = Some(number as u8);
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
        }

        let mut initial_state  = false;
        match &value["initial_state"] {
            serde_json::Value::Bool(value) => {
                initial_state = *value;
            }
            _ => {
                return Err("initial state is requiresd")
            }
        }

        let mut period: f32 = 10.0;
        match &value["period"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_f64() {
                    period = number as f32;
                    if period <= 0.0 {
                        return Err("period is invalid")
                    }
                }
            }
            _ => {
                return Err("period is required")
            }
        }

        let mut ratio: f32 = 1.0;
        match &value["ratio"] {
            serde_json::Value::Number(number) => {
                if let Some(number) = number.as_f64() {
                    ratio = number as f32;
                    if ratio < 0.0 || ratio > 1.0 {
                        return Err("ratio is invalid")
                    }
                }
            }
            _ => {
                return Err("ratio is invalid")
            }
        } 

        let gpio_pin = gpio_pin.unwrap_or_default();
        Ok ( Self {
            on_time_s,
            off_time_s,
            gpio_pin,
            initial_state,
            period,
            ratio,
            _empty: [b'\0'; 14],
        } )
    }


    pub fn new_from_bytes(
        bytes: [u8; SENSOR_SETTINGS_PARTITION_SIZE],
    ) -> TimedSwitch2SpecialConfiguration {
        let settings = bytes.as_ptr().cast::<TimedSwitch2SpecialConfiguration>();
        unsafe { *settings }
    }
}

pub struct TimedSwitch2 {
    general_config: SensorDriverGeneralConfiguration,
    special_config: TimedSwitch2SpecialConfiguration,
    state: u8, // 0: off, 1: on, other: invalid for now
    last_state_updated_at: i64,
    duty_cycle_state: bool,
    last_duty_cycle_update: u32,
    duty_cycle_on_time: u32,
    duty_cycle_off_time: u32,
}

impl TimedSwitch2 {
    pub fn new(
        general_config: SensorDriverGeneralConfiguration,
        special_config: TimedSwitch2SpecialConfiguration,
    ) -> Self {
        TimedSwitch2 {
            general_config,
            special_config,
            state: 0,
            last_state_updated_at: 0,
            duty_cycle_state: false,
            last_duty_cycle_update: 0,
            duty_cycle_on_time: 0,
            duty_cycle_off_time: 0,
        }
    }
}

impl SensorDriver for TimedSwitch2 {
    fn setup(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        board.set_gpio_pin_mode(self.special_config.gpio_pin, GpioMode::PushPullOutput);
        self.state = match self.special_config.initial_state {
            true => 1,
            false => 0,
        };
        board.write_gpio_pin(self.special_config.gpio_pin, self.state == 1);
        let timestamp = board.timestamp();
        self.last_state_updated_at = timestamp;
        self.duty_cycle_state = self.state == 1;
        let millis = board.millis();
        self.last_duty_cycle_update = millis;
        self.duty_cycle_on_time = (self.special_config.period * self.special_config.ratio * 1000.0) as u32;
        self.duty_cycle_off_time = (self.special_config.period * 1000.0) as u32 - self.duty_cycle_on_time;
    }

    fn get_requested_gpios(&self) -> super::resources::gpio::GpioRequest {
        let mut gpio_request = super::resources::gpio::GpioRequest::none();
        gpio_request.use_pin(self.special_config.gpio_pin); 
        gpio_request
    }

    getters!();
    

    fn get_measured_parameter_count(&mut self) -> usize {
        return 1;
    }

    fn get_measured_parameter_value(&mut self, index: usize) -> Result<f64, ()> {
        Ok(self.state as f64)
    }

    fn get_measured_parameter_identifier(&mut self, index: usize) -> [u8;16] {
        let mut rval = [0u8;16];
        rval[0..7].clone_from_slice("heater\0".as_bytes());
        return rval;
    }

    fn take_measurement(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        //
    }

    fn update_actuators(&mut self, board: &mut dyn rriv_board::SensorDriverServices) {
        let timestamp = board.timestamp();
        let millis = board.millis();

        let mut gpio_state = false;
        let mut toggle_state = false;
        if self.state == 0 {
            // heater is off
            if timestamp - self.special_config.off_time_s as i64 > self.last_state_updated_at {
                rprintln!("state is 0, toggle triggered");
                toggle_state = true;
                gpio_state = true;
                self.state = 1;
                self.last_duty_cycle_update = millis;
                self.duty_cycle_state = true;
            }
        } else if self.state == 1 {
            // heater is on

            // duty cycle implementation
            let elapsed: i32 = millis as i32 - self.last_duty_cycle_update as i32;
            let mut new_elapsed: u32 = elapsed as u32;
            if elapsed < 0 {
                // millis overflowed
                new_elapsed = MAX_MILLIS - self.last_duty_cycle_update + millis;
            }
            
            if self.duty_cycle_state == true && new_elapsed > self.duty_cycle_on_time {
                toggle_state = true;
                gpio_state = false;
                self.last_duty_cycle_update = millis;
                self.duty_cycle_state  = false;
            } else if self.duty_cycle_state == false && new_elapsed > self.duty_cycle_off_time {
                toggle_state = true;
                gpio_state = true;
                self.last_duty_cycle_update = millis;
                self.duty_cycle_state  = true;
            } 

            // end of on_time (outer cycle)
            if timestamp - self.special_config.on_time_s as i64 > self.last_state_updated_at {
                rprintln!("state is 1, toggle triggered");
                toggle_state = true;
                gpio_state = false;
                self.state = 0;
            }
        }

        if toggle_state { 
            rprintln!("toggled to {}", gpio_state);
            // rprintln!("on_time: {}, ratio: {}, period: {}\nduty cycle on time: {}, off time: {}", self.special_config.on_time_s, self.special_config.ratio, self.special_config.period, self.duty_cycle_on_time, self.duty_cycle_off_time);
            board.write_gpio_pin(self.special_config.gpio_pin, gpio_state);
            self.last_state_updated_at = timestamp;
        }
    }
    
    fn get_configuration_bytes(&self, storage: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE]) {
        
        let generic_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&self.general_config) };
        let special_settings_bytes: &[u8] = unsafe { any_as_u8_slice(&self.special_config) };

        copy_config_into_partition(0, generic_settings_bytes, storage);
        copy_config_into_partition(1, special_settings_bytes, storage);
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

        json!({
            "id" : sensor_id,
            "type" : sensor_name,
            "on_time_s": self.special_config.on_time_s,
            "off_time_s": self.special_config.off_time_s,
            "gpio_pin": self.special_config.gpio_pin,
            "initial_state" : self.special_config.initial_state
        })
    }
    
    fn fit(&mut self, pairs: &[CalibrationPair]) -> Result<(), ()> {
        todo!()
    }
    
    fn clear_calibration(&mut self) {
        todo!()
    }
}
