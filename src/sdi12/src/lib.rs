#![no_std]

use rriv_board::gpio;

const SDI12_TIMING_TOLERANCE: u16 = 400;
const SDI12_BREAK_DURATION_US: u16 = 12000;
const SDI12_MARK_DURATION_US: u16 = 8333;
const SDI12_TICKS_PER_BIT: u16 = 8333; // 1 bit duration at 1200 baud
// const SDI12_TIMEOUT : u32 = 100; // timeout for reading response in milliseconds
const SDI12_GAP: u16 = 5000;
pub const SDI12_BUFFER_SIZE: usize = 100; // size of the buffer for reading responses
pub const SDI12_COMMAND_SIZE: usize = 10;

#[derive(PartialEq, Debug)]
pub enum SDIPinState {
    Sdi12Disabled,         // SDI-12 is disabled, pin mode INPUT, interrupts disabled for the pin
    Sdi12Enabled,          // SDI-12 is enabled, pin mode INPUT, interrupts disabled for the pin 
    Sdi12Holding,          // The line is being held LOW, pin mode OUTPUT, interrupts disabled for the pin
    Sdi12Transmitting,     // Data is being transmitted by the SDI-12 master, pin mode OUTPUT, interrupts disabled for the pin
    Sdi12Listening         // The SDI-12 master is listening for a response from the slave, pin mode INPUT, interrupts enabled for the pin
}

pub trait BoardForSDI12 {
    fn write(&mut self, value: bool);
    fn read(&mut self) -> bool;
    fn delay_us(&mut self, us: u16);
    fn pin_mode(&mut self, mode: gpio::GpioMode);
    fn millis(&mut self) -> u32;
}

#[allow(non_camel_case_types)]
pub struct SDI12_MResponse {
    pub address: char,
    pub ttt: u32,
    pub n: u8
}

#[allow(non_camel_case_types)]
pub struct SDI12_Dresponse {
    pub address: char,
    pub data: [f32; 9],
    pub count: u8,
    pub terminate: bool,
    pub last_d_ind: u8
}

pub struct SDI12 <B> {
    state: SDIPinState,
    sdi12_board: B,
    timeout_counter: u32
}

impl<B> SDI12<B> where B: BoardForSDI12,
{
    pub fn new(sdi12_board: B) -> Self
    {
        SDI12 {
            state: SDIPinState::Sdi12Enabled,
            sdi12_board: sdi12_board,
            timeout_counter: 0
        }
    }

    pub fn set_state(&mut self, state: SDIPinState) {
        // set the digital pin to the specified value
        if self.state != state {
            self.state = state;
            match self.state {
                SDIPinState::Sdi12Transmitting => {
                    // set pin mode to OUTPUT
                    self.sdi12_board.pin_mode(gpio::GpioMode::PushPullOutput);
                },
                SDIPinState::Sdi12Listening => {
                    // set pin mode to INPUT
                    self.sdi12_board.pin_mode(gpio::GpioMode::PullDownInput);
                }
                _ => {
                    // For SDI12_HOLDING, SDI12_DISABLED and SDI12_ENABLED, set pin mode to INPUT
                    self.sdi12_board.pin_mode(gpio::GpioMode::PullDownInput);
                }
            }
        }
    }

    pub fn send_break(&mut self) {

        self.set_state(SDIPinState::Sdi12Transmitting);
        
        // Hold it HIGH for 12 ms
        self.sdi12_board.write(true);
        self.sdi12_board.delay_us(SDI12_BREAK_DURATION_US);
        
        // Marking by holding it LOW for 8.33 ms
        self.sdi12_board.write(false);
        self.sdi12_board.delay_us(SDI12_MARK_DURATION_US);
    }

    pub fn receive_break(&mut self) -> bool {   
        self.set_state(SDIPinState::Sdi12Listening);
        if self.sdi12_board.read() {
            // defmt::println!("Start for 12ms");
            self.sdi12_board.delay_us(SDI12_BREAK_DURATION_US - SDI12_TIMING_TOLERANCE);
            let mut iter_count = 0;
            while self.sdi12_board.read() {
                // defmt::println!("iter_count: {}", iter_count);
                if iter_count > 10 {
                    defmt::println!("Timeout! line is not falling low");
                    return false;
                }
                iter_count += 1;
                self.sdi12_board.delay_us(SDI12_TIMING_TOLERANCE);
            }
            // // check after every 1ms for 12 times to see if it is a valid break
            // for _ in 0..12 {
            //     self.sdi12_board.delay_us(1000); // Wait 1ms
                
            //     if self.sdi12_board.read() == false {
            //         defmt::println!("Line dropped low before 12ms");
            //         return false;
            //     }
            // }
            // defmt::println!("Valid break, waiting...");
            // // wait for the line to drop LOW
            // let mut iter_count = 0;
            // while self.sdi12_board.read() == true {
            //     if iter_count > 1 {
            //         defmt::println!("Timeout! line is not falling low");
            //         return false;
            //     }
            //     self.sdi12_board.delay_us(SDI12_TIMING_TOLERANCE);
            //     iter_count += 1;
            // }
            self.sdi12_board.delay_us(SDI12_MARK_DURATION_US - 2 * SDI12_TIMING_TOLERANCE);
            if self.sdi12_board.read() {
                defmt::println!("Invalid marking");
                return false;
            }
            self.sdi12_board.delay_us(SDI12_TIMING_TOLERANCE);
            // defmt::println!("Valid marking");
            // for _ in 0..8 {
            //     self.sdi12_board.delay_us(1000);
            //     if self.sdi12_board.read() {
            //         defmt::println!("line went high too early, invalid marking");
            //         return false;
            //     }
            // }
            self.sdi12_board.delay_us(SDI12_TIMING_TOLERANCE);
            return true;
        }
        return false;
    }

    pub fn write_char(&mut self, c: char) {
        // sdi-12 write character implementation
        // convert char to byte and write it bit by bit, LSB first, with a start bit and a stop bit
        let mut byte = c as u8;
        let parity = parity_bit(byte);
        byte |= (parity as u8) << 7;    // add parity bit as the most significant bit

        // start bit
        self.sdi12_board.write(true);
        self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
        // data bits
        for i in 0..8 {
            let bit = (byte >> i) & 1;
            self.sdi12_board.write(bit == 0);
            self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
        }
        // stop bit
        self.sdi12_board.write(false);
        self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
    }

    pub fn read_char(&mut self) -> Option<char> {
        let mut iter_count = 0;
        while self.sdi12_board.read() == false {
            // let millis = self.sdi12_board.millis();
            // let elapsed: i32 = millis as i32 - self.timeout_counter as i32;
            // let mut new_elapsed: u32 = elapsed as u32;
            // if elapsed < 0 {
            //     new_elapsed = 65535 - self.timeout_counter + millis;
            // }
            // defmt::println!("millis {}, elapsed time: {}, timeout: {}", millis, new_elapsed, self.timeout_counter);
            // if new_elapsed as u32 > SDI12_TIMEOUT {
            //     return None; // SDI12_timeout
            // }
            if iter_count > 100 {
                return None;
            }
            iter_count += 1;
            self.sdi12_board.delay_us(1000);
        }
        // defmt::println!("detected a start bit");
        self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT / 2);

        if self.sdi12_board.read() == false {
            return None; 
        }

        let mut byte: u8 = 0;

        for i in 0..8 {
            self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
            let bit_value = match self.sdi12_board.read() {
                true => 0,
                false => 1
            };
            byte |= bit_value << i;
        }

        self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
        let stop_bit = self.sdi12_board.read();

        if stop_bit {
            return None;
        }

        let character_data = byte & 0x7F;

        // Verify the parity bit here before returning)
        let parity_bit_received = (byte >> 7) & 1 == 1;
        let parity_bit_calculated = parity_bit(character_data);
        if parity_bit_received != parity_bit_calculated {
            return None;
        }

        Some(character_data as char)
    }

    pub fn send_command(&mut self, command: [char; SDI12_COMMAND_SIZE]) {
        // sdi-12 implementation
        self.set_state(SDIPinState::Sdi12Transmitting);
        for c in command.iter() {
            self.write_char(*c);
            if *c == '!' {
                break; // stop at termination character
            }
            // self.sdi12_board.delay_us(SDI12_GAP);
        }
        self.set_state(SDIPinState::Sdi12Listening);
    }

    pub fn read_command(&mut self) -> [char; SDI12_COMMAND_SIZE] {
        self.set_state(SDIPinState::Sdi12Listening);
        let mut buffer: [char; SDI12_COMMAND_SIZE] = ['\0'; SDI12_COMMAND_SIZE];
        let mut bytes_read = 0;

        while bytes_read < SDI12_COMMAND_SIZE {
            self.timeout_counter = self.sdi12_board.millis();
            match self.read_char() {
                Some(byte) => {
                    // store the byte in the response buffer
                    buffer[bytes_read] = byte;
                    bytes_read += 1;
                    if byte == '!' {
                        defmt::println!("buffer[{}] = {}", bytes_read, byte);
                        break;
                    }
                },
                None => {
                    defmt::println!("Timeout error");
                    defmt::println!("buffer[{}] = {}", bytes_read, buffer);
                    break; // SDI12_timeout or error
                }
            }
        }
        self.sdi12_board.delay_us(SDI12_GAP);

        buffer
    }

    pub fn send_response(&mut self, data: [char; SDI12_BUFFER_SIZE]) {
        self.set_state(SDIPinState::Sdi12Transmitting);
        for c in data.iter() {
            self.write_char(*c);
            if *c == '\n' {
                break; // stop at termination character
            }
            // self.sdi12_board.delay_us(SDI12_GAP);
        }
        self.set_state(SDIPinState::Sdi12Listening);
    }

    pub fn read_response(&mut self) -> [char; SDI12_BUFFER_SIZE] {
        // sdi-12 implementation
        self.set_state(SDIPinState::Sdi12Listening);
        defmt::println!("Reading response...");
        let mut buffer: [char; SDI12_BUFFER_SIZE] = ['\0'; SDI12_BUFFER_SIZE];
        let mut bytes_read = 0;
        while bytes_read < SDI12_BUFFER_SIZE {
            self.timeout_counter = self.sdi12_board.millis();
            match self.read_char() {
                Some(byte) => {
                    // store the byte in the response buffer
                    buffer[bytes_read] = byte;
                    bytes_read += 1;
                    // defmt::println!("buffer[{}] = {}", bytes_read, byte);
                    if byte == '\n' {
                        break;
                    }
                },
                None => {
                    defmt::println!("Timeout SDI12");
                    break; // SDI12_timeout or error
                }
            }
        }
        self.sdi12_board.delay_us(SDI12_GAP);
        buffer
    }

    pub fn parse_data(&mut self, response: &[char]) -> ([f32; 9], u8) {
        let mut count = 0;
        let mut temp_buf = [0u8; 16]; 
        let mut temp_len = 0;

        let mut data: [f32; 9] = [0_f32; 9];

        for &c in response {
            if c == '+' || c == '-' || c == '\r' || c == '\n' {
                if temp_len > 0 {
                    // Convert to a string slice
                    if let Ok(s) = core::str::from_utf8(&temp_buf[..temp_len]) {
                        // Parse the float
                        if let Ok(val) = s.parse::<f32>() {
                            data[count] = val;
                            count += 1;
                        }
                    }
                }
                temp_len = 0;
                if c == '\r' || c == '\n' {
                    break;
                }
                
                if c == '+' || c == '-' {
                    temp_buf[0] = c as u8;
                    temp_len = 1;
                }
                
            } 
            else if c.is_ascii() && temp_len < temp_buf.len() {
                temp_buf[temp_len] = c as u8;
                temp_len += 1;
            }
        }

        (data, count as u8)
    }

}

// Helper functions
pub fn parity_bit(byte: u8) -> bool {
    // returns the parity bit for the given byte, true for odd parity, false for even parity
    let mut count = 0;
    for i in 0..8 {
        if (byte >> i) & 1 == 1 {
            count += 1;
        }
    }
    count % 2 == 1
}