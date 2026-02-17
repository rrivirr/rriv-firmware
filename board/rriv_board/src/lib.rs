#![cfg_attr(not(test), no_std)]
extern crate alloc;
use alloc::boxed::Box;
use core::fmt;


pub mod gpio;
pub mod hardware_error;

use crate::{gpio::GpioMode, hardware_error::HardwareError};

pub const EEPROM_DATALOGGER_SETTINGS_SIZE: usize = 64;
pub const EEPROM_SENSOR_SETTINGS_SIZE: usize = 64;
pub const EEPROM_SERIAL_NUMBER_SIZE: usize = 5;


#[cfg(feature = "24LC08")]
pub const EEPROM_TOTAL_SENSOR_SLOTS: usize = 12;

#[cfg(feature = "24LC01")]
pub const EEPROM_TOTAL_SENSOR_SLOTS: usize = 2;

pub trait RXProcessor: Send + Sync {
    fn process_byte(&mut self, byte: u8);
}

pub enum SerialRxPeripheral{
    CommandSerial,
    SerialPeripheral1,
    SerialPeripheral2
}


pub trait RRIVBoard: Send {

    // Run loop services
    fn run_loop_iteration(&mut self);

    // Core Services
    fn set_serial_rx_processor(&mut self, peripheral: SerialRxPeripheral, processor: Box<&'static mut dyn RXProcessor>);
    fn critical_section(&self, f: fn());
    // Storage Services
    fn store_datalogger_settings(&mut self, bytes: &[u8;EEPROM_DATALOGGER_SETTINGS_SIZE]);
    fn retrieve_datalogger_settings(&mut self, buffer: &mut [u8;EEPROM_DATALOGGER_SETTINGS_SIZE]);
    fn store_sensor_settings(&mut self, slot: u8, bytes: &[u8; EEPROM_SENSOR_SETTINGS_SIZE] );
    fn retrieve_sensor_settings(&mut self, buffer: &mut [u8; EEPROM_SENSOR_SETTINGS_SIZE * EEPROM_TOTAL_SENSOR_SLOTS]);

    // Modes
    fn set_debug(&mut self, debug: bool);

    // Data Logging
    fn write_log_file(&mut self, args: fmt::Arguments);
    fn flush_log_file(&mut self);


    // Time
    fn set_epoch(&mut self, epoch: i64);
    fn epoch_timestamp(&mut self) -> i64;
    fn get_millis(&mut self) -> u32;

    // Board Services Used by Control Logic and Drivers
    fn usb_serial_send(&mut self, arg: fmt::Arguments);
                // TODO: give his a more unique name specifying that it's used to talk with the serial rrivctl interface
                // maybe rrivctl_send
    fn usart_send(&mut self, bytes: &[u8]);
    fn rs485_send(&mut self, message : &[u8]);
    fn serial_debug(&mut self, args: fmt::Arguments);
    fn delay_ms(&mut self, ms: u16);
    fn timestamp(&mut self) -> i64;
    fn millis(&mut self) -> u32;


    fn get_battery_level(&mut self) -> i16;

    fn sleep(&mut self);

    // low level board functionality
    // for debugging and basic operation
    fn dump_eeprom(&mut self);
    fn get_uid(&mut self) -> [u8; 12];
    fn set_serial_number(&mut self, serial_number: [u8;5]) -> bool;
    fn get_serial_number(&mut self) -> [u8;5];
    
    // fn subsystem(&mut self, ...)  //TODO: custom commands to the board subsystems, use a tokenized rather than json format

    // future functions for ADC interface
    // fn get_adc_capabilities(&mut self); // minimum functionality return of adcs
    // fn get power on status of each adc
    // fn change power on status of each adc
    // fn query adc by index
    
    fn query_internal_adc(&mut self, port: u8) -> u16;
    fn query_external_adc(&mut self, port: u8) -> u16;
    fn ic2_read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), ()>;
    fn ic2_write(&mut self, addr: u8, message: &[u8]) -> Result<(), ()>;
    fn ic2_write_read(&mut self, addr: u8, message: &[u8], buffer: &mut [u8]) -> Result<(), ()>;


    fn write_gpio_pin(&mut self, pin: u8, value: bool);
    fn write_pwm_pin_duty(&mut self, value: u8);
    fn read_gpio_pin(&mut self, pin: u8) -> Result<bool, ()>;

    fn set_gpio_pin_mode(&mut self, pin: u8, mode: GpioMode);

    fn one_wire_send_command(&mut self, command: u8, address: u64);
    fn one_wire_reset(&mut self);
    fn one_wire_skip_address(&mut self);
    fn one_wire_write_byte(&mut self, byte: u8);
    fn one_wire_match_address(&mut self, address: u64);
    fn one_wire_read_bytes(&mut self, output: &mut [u8]) -> Result<(), ()>;
    fn one_wire_bus_start_search(&mut self);
    fn one_wire_bus_search(&mut self) -> Option<u64>;

    fn read_temp_adc(&mut self) -> i32;
    fn disable_interrupts(&self);
    fn enable_interrupts(&self);

    fn get_errors(&self) -> [HardwareError; 5]; // return up to 5 hardware errors currently raised
    fn error_alarm(&mut self); // activate a generic error alarm, normally an LED
    
}


pub trait RRIVBoardBuilder {
    fn setup(&mut self);
    // fn build(self) -> dyn RRIVBoard;
}