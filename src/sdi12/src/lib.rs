


pub trait digital_pin {
    pub fn write(value);
    pub fn read() -> bool;
}

pub trait delay_us {
    pub fn delay(ms: u32);
}


pub fn send_command(&str, &mut impl digital_pin, &mut impl delay_us) -> [u8;100] {
    // sdi-12 implementation
}