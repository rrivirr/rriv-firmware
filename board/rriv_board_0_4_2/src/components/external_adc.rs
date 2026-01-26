use crate::*;
use bitfield_struct::bitfield;

const ADC_I2C_ADDRESS: u8 = 0x2F;
const ADC_CONVERSION_RESULT_REGISTER_ADDRESS: u8 = 0x00;
const ADC_CHANNEL_REGISTER_ADDRESS: u8 = 0x01;
const ADC_CONFIGURATION_REGISTER_ADDRESS: u8 = 0x02;

#[bitfield(u16)]
struct ConversionResultRegister {
    #[bits(12)]
    conv_result: u16,

    #[bits(1)]
    alert: bool,

    #[bits(2)]
    channel: u8,

    #[bits(1)]
    reserved: u8,
}

impl ConversionResultRegister {
    fn from_be_bytes(bytes: [u8; 2]) -> ConversionResultRegister {
        let u16_value: u16 = ((bytes[0] as u16) << 8) | bytes[1] as u16;
        ConversionResultRegister::from_bits(u16_value)
    }
}

#[bitfield(u8)] // bit order is opposite on MCU vs ADC chip
struct ChannelRegister {
    #[bits(1, default = false)]
    channel_0: bool,

    #[bits(1, default = false)]
    channel_1: bool,

    #[bits(1, default = false)]
    channel_2: bool,

    #[bits(1, default = false)]
    channel_3: bool,

    #[bits(4, access = RO, default = 0)]
    reserved: u8,
}

fn build_channel_register_set_command(register: ChannelRegister) -> [u8; 2] {
    let register_bytes = unsafe { any_as_u8_slice(&register) };
    let data: [u8; 2] = [ADC_CHANNEL_REGISTER_ADDRESS, register_bytes[0]];
    return data;
}

#[bitfield(u16)]
struct ConfigurationRegister {
    #[bits(2, default = 0)]
    p_down: u8,

    #[bits(1, default = false)]
    gpo1: bool,

    #[bits(1, default = false)]
    alert_pol_or_gpo0: bool,

    #[bits(1, default = false)]
    alert_en_or_gpoo: bool,

    #[bits(1, default = false)]
    busy: bool,

    #[bits(2, default = 3)]
    cycle_timer: u8,

    #[bits(1, default = false)]
    auto: bool,

    #[bits(1, default = false)]
    swrs: bool,

    #[bits(1, default = true)]
    cmd: bool,

    #[bits(1, default = false)]
    fltr: bool,

    #[bits(2, access = RO, default = 0)]
    reserved: u8,

    #[bits(1, default = false)]
    gpo2: bool,

    #[bits(1, default = false)]
    alert_drive_type: bool,
}

fn build_configuration_register_set_command(register: ConfigurationRegister) -> [u8; 3] {
    let register_bytes = unsafe { any_as_u8_slice(&register) };
    let data: [u8; 3] = [
        ADC_CONFIGURATION_REGISTER_ADDRESS,
        register_bytes[1], // MSB (I2C is Big Ending Byte Order)
        register_bytes[0], // LSB
    ];
    return data;
}

pub struct ExternalAdc {
    pins: pin_groups::ExternalAdcPins,
}

impl ExternalAdc {
    pub fn new(pins: pin_groups::ExternalAdcPins) -> Self {
        return ExternalAdc { pins };
    }

    pub fn start(&mut self, delay: &mut impl embedded_hal::blocking::delay::DelayMs<u8>) {
        self.enable(delay);
        self.reset(delay);
    }

    pub fn reset(&mut self, delay: &mut impl embedded_hal::blocking::delay::DelayMs<u8>) {
        delay.delay_ms(1_u8); // delay > 50ns before applying ADC reset
        self.pins.reset.set_low();
        delay.delay_ms(1_u8); // delay > 10ns after starting ADC reset
        self.pins.reset.set_high();
        delay.delay_ms(100_u8); // let the chip start up
    }

    // read a single channel
    pub fn read_single_channel(&mut self, i2c: &mut BoardI2c1, channel: u8) -> u16 {
        let mut channel_register = ChannelRegister::new();
        match channel {
            1 => channel_register.set_channel_0(true),
            2 => channel_register.set_channel_1(true),
            3 => channel_register.set_channel_2(true),
            4 => channel_register.set_channel_3(true),
            _ => {}
        };


        let channel_command = build_channel_register_set_command(channel_register);
        defmt::println!("set channel reg: {}", channel_command[1]);
        self.send_i2c(i2c, &channel_command); // writing to the channel register restarts the channel cycle.

        let mut buffer: [u8; 2] = [0; 2];
        self.read_i2c(i2c, ADC_CONVERSION_RESULT_REGISTER_ADDRESS, &mut buffer);
        // MSB and LSB need to be swapped in order to change to little endian
        let conversion_result = ConversionResultRegister::from_be_bytes(buffer);
        let value = conversion_result.conv_result();
        defmt::println!(
            "EXADC: conversion result {} : {}   {:?}",
            conversion_result.channel(),
            value,
            buffer
        );
        value
    }

    pub fn enable(&mut self, delay: &mut impl embedded_hal::blocking::delay::DelayMs<u8>) {
        self.pins.enable.set_low();
        delay.delay_ms(250); // let the chip get power
    }

    pub fn disable(&mut self, delay: &mut impl embedded_hal::blocking::delay::DelayMs<u8>) {
        self.pins.enable.set_high();
        delay.delay_ms(250); // let the chip power down
    }

    pub fn configure(&mut self, i2c: &mut BoardI2c1) {
        let configuration_register = ConfigurationRegister::new();
        let configuration_command =
            build_configuration_register_set_command(configuration_register); // swaps to big endian
        self.send_i2c(i2c, &configuration_command);
        defmt::println!(
            "send config reg: {:X} {:08b} {:08b} {:08b}",
            configuration_command,
            configuration_command[0],
            configuration_command[1],
            configuration_command[2]
        );
        let mut buffer: [u8; 2] = [0; 2];
        self.read_i2c(i2c, ADC_CONFIGURATION_REGISTER_ADDRESS, &mut buffer);
        defmt::println!("config reg: {:X}", buffer);
    }

    fn send_i2c(&mut self, i2c: &mut BoardI2c1, bytes: &[u8]) {
        defmt::println!("send i2c {:X}", bytes);
        match i2c.write(ADC_I2C_ADDRESS, bytes) {
            Ok(_) => {}
            Err(err) => defmt::println!("{:?}", defmt::Debug2Format(&err)),
        }
    }

    fn read_i2c(&mut self, i2c: &mut BoardI2c1, register_address: u8, buffer: &mut [u8]) {
        let address_bytes: [u8; 1] = [register_address];
        defmt::println!("read i2c {:X}", address_bytes);
        match i2c.write_read(ADC_I2C_ADDRESS, &address_bytes, buffer) {
            Ok(_) => {}
            Err(err) => defmt::println!("{=?}", defmt::Debug2Format(&err)),
        }
    }
}
