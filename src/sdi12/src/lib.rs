#![no_std]

use rriv_board::gpio;

// SDI-12 Timing Constants (in microseconds)
const SDI12_TIMING_TOLERANCE: u16 = 400;
const SDI12_BREAK_DURATION_US: u16 = 12100;
const SDI12_MARK_DURATION_US: u16 = 8400;
const SDI12_TICKS_PER_BIT: u16 = 833; // 1 bit duration at 1200 baud
// const SDI12_TIMEOUT : u32 = 100; // timeout for reading response in milliseconds
const SDI12_GAP: u16 = 5000;
pub const SDI12_BUFFER_SIZE: usize = 100; // size of the buffer for reading responses
pub const SDI12_COMMAND_SIZE: usize = 10;

// RX BUFFER DATA
const WAITING_FOR_START_BIT: u8 = 255;
const WAITING_FOR_BREAK: u8 = 254;
const WAITING_FOR_MARK: u8 = 253;
const WAITING_FOR_START_AFTER_BREAK: u8 = 252;

static mut LAST_TICK: u32 = 0;
static mut RX_STATE: u8 = WAITING_FOR_START_BIT;      // 255 means idle state, 0-7 means receiving bits for a byte, 8 means waiting for stop bit
static mut RX_VALUE: u8 = 0x00;
static mut RX_MASK: u8 = 0x01;

static mut RX_BUFFER: [char; SDI12_BUFFER_SIZE] = ['\0'; SDI12_BUFFER_SIZE];
static mut RX_HEAD: usize = 0;
static mut RX_TAIL: usize = 0;

static mut RECEIVED_BREAK: bool = false;

#[derive(PartialEq, Debug)]
pub enum SDIPinState {
    Sdi12Sleep,            // SDI-12 slave is sleeping, pin mode INPUT, interrupts enabled for the pin
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
    fn enable_interrupt(&mut self);
    fn disable_interrupt(&mut self);
    fn get_current_time(&self) -> u32;  // microseconds
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
        self.state = state;
        match self.state {
            SDIPinState::Sdi12Transmitting => {
                // set pin mode to OUTPUT
                self.sdi12_board.pin_mode(gpio::GpioMode::PushPullOutput);
                self.sdi12_board.disable_interrupt();
            },
            SDIPinState::Sdi12Listening => {
                // set pin mode to INPUT
                self.sdi12_board.pin_mode(gpio::GpioMode::PullDownInput);        
                unsafe { 
                    RX_STATE = WAITING_FOR_START_BIT;     // reset the interrupt state variables
                    LAST_TICK = self.sdi12_board.get_current_time();
                }
                self.sdi12_board.enable_interrupt();
                defmt::println!("State changed to: Sdi12Listening");      
            }
            SDIPinState::Sdi12Sleep => {
                self.sdi12_board.pin_mode(gpio::GpioMode::PullDownInput);
                unsafe { 
                    RX_STATE = WAITING_FOR_BREAK;     // reset the interrupt state variables
                    RECEIVED_BREAK = false;
                    LAST_TICK = self.sdi12_board.get_current_time();
                }  
                self.sdi12_board.enable_interrupt();
                defmt::println!("State changed to: Sdi12Sleep");
            }
            _ => {
                // For SDI12_HOLDING and SDI12_ENABLED, set pin mode to INPUT
                self.sdi12_board.pin_mode(gpio::GpioMode::PullDownInput);
                self.sdi12_board.disable_interrupt();
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
            self.sdi12_board.delay_us(SDI12_MARK_DURATION_US - 2 * SDI12_TIMING_TOLERANCE);
            if self.sdi12_board.read() {
                defmt::println!("Invalid marking");
                return false;
            }
            self.sdi12_board.delay_us(SDI12_TIMING_TOLERANCE);
            self.sdi12_board.delay_us(SDI12_TIMING_TOLERANCE);
            return true;
        }
        return false;
    }

    // pub fn write_char(&mut self, c: char) {
    //     // sdi-12 write character implementation
    //     // convert char to byte and write it bit by bit, LSB first, with a start bit and a stop bit
    //     let mut byte = c as u8;
    //     let parity = parity_bit(byte);
    //     byte |= (parity as u8) << 7;    // add parity bit as the most significant bit

    //     // start bit
    //     self.sdi12_board.write(true);
    //     self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
    //     // data bits
    //     for i in 0..8 {
    //         let bit = (byte >> i) & 1;
    //         self.sdi12_board.write(bit == 0);
    //         self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
    //     }
    //     // stop bit
    //     self.sdi12_board.write(false);
    //     self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
    // }

    pub fn write_char(&mut self, c: char) {
        let mut out_char = c as u8;
        let ticks_per_bit: u32 = SDI12_TICKS_PER_BIT as u32; // SDI12_TICKS_PER_BIT
        
        let mut start_time = self.sdi12_board.get_current_time();

        // 1. Immediately get going on the start bit (HIGH)
        self.sdi12_board.write(true); 

        // 2. Calculate parity while the start bit is physically holding the line
        let parity_bit = (out_char.count_ones() as u8) & 1; 
        out_char |= parity_bit << 7; 

        // 3. Calculate the position of the last bit that is a 0/HIGH.
        let mut last_high_bit: u8 = 9;
        let mut msb_mask: u8 = 0x80;
        
        while (msb_mask & out_char) != 0 {
            last_high_bit -= 1;
            msb_mask >>= 1;
        }

        // 4. Hold the line for the rest of the start bit duration
        while self.sdi12_board.get_current_time().wrapping_sub(start_time) < ticks_per_bit {}
        start_time = start_time.wrapping_add(ticks_per_bit); 

        // 5. Send data bits until the last bit different from marking (LOW)
        let mut current_tx_bit_num: u8 = 1;
        
        while current_tx_bit_num < last_high_bit {
            let bit_value = out_char & 0x01;
            
            if bit_value != 0 {
                self.sdi12_board.write(false); // LOW for 1's
            } else {
                self.sdi12_board.write(true);  // HIGH for 0's
            }

            // Wait for bit duration
            while self.sdi12_board.get_current_time().wrapping_sub(start_time) < ticks_per_bit {}
            start_time = start_time.wrapping_add(ticks_per_bit);

            out_char >>= 1; 
            current_tx_bit_num += 1;
        }

        // 6. Set the line LOW for the remaining 1's AND the stop bit
        self.sdi12_board.write(false);

        // =========================================================
        // ARCHITECTURE NOTE: 
        // In the C++ version, interrupts were turned back on right here.
        // Since your caller manages interrupts, the CPU will remain 
        // "deaf" to interrupts during this final sleep block as well. 
        // =========================================================

        // 7. Hold the line LOW until the end of the 10th bit
        let remaining_bits = 10 - last_high_bit;
        let bit_time_remaining = ticks_per_bit * (remaining_bits as u32);
        
        // Notice we use `start_time` here, so the trailing bits stay locked to the grid
        while self.sdi12_board.get_current_time().wrapping_sub(start_time) < bit_time_remaining {}
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
        self.sdi12_board.delay_us(SDI12_GAP);
        for c in command.iter() {
            self.write_char(*c);
            if *c == '!' {
                break; // stop at termination character
            }
            // self.sdi12_board.delay_us(SDI12_GAP);
        }
        self.sdi12_board.delay_us(SDI12_TICKS_PER_BIT);
        defmt::println!("Command sent, switching to listening mode");
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
        self.sdi12_board.delay_us(SDI12_GAP);
        for c in data.iter() {
            self.write_char(*c);
            if *c == '\n' {
                break; // stop at termination character
            }
            // self.sdi12_board.delay_us(SDI12_GAP);
        }
        // self.set_state(SDIPinState::Sdi12Listening);
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

    pub fn available(&mut self) -> usize {
        unsafe {
            (RX_TAIL + SDI12_BUFFER_SIZE - RX_HEAD) % SDI12_BUFFER_SIZE
        }
    }

    pub fn read(&mut self) -> Option<char> {
        unsafe {
            if RX_HEAD == RX_TAIL {
                None
            }
            else {
                let c = RX_BUFFER[RX_HEAD];
                RX_HEAD = (RX_HEAD + 1) % SDI12_BUFFER_SIZE;
                Some(c)
            }
        }
    }

    pub fn peek(&mut self) -> Option<char> {
        unsafe {
            if RX_HEAD == RX_TAIL {
                None
            }
            else {
                Some(RX_BUFFER[RX_HEAD])
            }
        }
    }

    pub fn clear_buffer(&mut self) {
        unsafe {
            RX_HEAD = 0;
            RX_TAIL = 0;
        }
    }

    pub fn sleep(&mut self) {
        self.set_state(SDIPinState::Sdi12Sleep);
    }

    pub fn awake(&mut self) -> bool {
        let received_break = unsafe { RECEIVED_BREAK };
        received_break
    }
}

// Helper functions
fn parity_bit(byte: u8) -> bool {
    // returns the parity bit for the given byte, true for odd parity, false for even parity
    let mut count = 0;
    for i in 0..8 {
        if (byte >> i) & 1 == 1 {
            count += 1;
        }
    }
    count % 2 == 1
}

fn num_bits_passed(dt: u32) -> u16 {
    ((dt + 2) / SDI12_TICKS_PER_BIT as u32) as u16
}

// Interrupt Handlers
fn start_char() {
    unsafe {
        RX_STATE = 0;
        RX_VALUE = 0x00;
        RX_MASK = 0x01;
    }
}

fn char_to_buffer(c: char) {
    unsafe {
        if (RX_TAIL + 1) % SDI12_BUFFER_SIZE == RX_HEAD {
            defmt::println!("Buffer overflow, discarding data");
        }
        else {
            RX_BUFFER[RX_TAIL] = c;
            RX_TAIL = (RX_TAIL + 1) % SDI12_BUFFER_SIZE;
        }
    }
}

pub fn datalogger_interrupt_handler(now: u32, gpio_state: bool) {

    let dt = now.wrapping_sub(unsafe { LAST_TICK });
    let bits_passed = num_bits_passed(dt);
    unsafe { LAST_TICK = now; }
    let mut rx_state = unsafe { RX_STATE };
    let mut rx_value = unsafe { RX_VALUE };
    let mut rx_mask = unsafe { RX_MASK };

    // defmt::println!("Interrupt! gpio_state: {}, dt: {}, bits_passed: {}, rx_state: {}", gpio_state, dt, bits_passed, rx_state);

    // Waiting for start bit
    if rx_state == WAITING_FOR_START_BIT {
        if gpio_state == true {
            // it is a start bit
            start_char();
        }
        return;
    }
    else {
        // Receiving bits for a byte
        let bits_left = 9 - rx_state;

        let next_char_started = bits_passed > bits_left as u16;
        let mut bits_to_process = if next_char_started { bits_left } else { bits_passed as u8 };
        rx_state += bits_to_process;

        if gpio_state == true {
            while bits_to_process > 0 {
                rx_value |= rx_mask;      // all the LOW bits are stored as 1
                rx_mask <<= 1;
                bits_to_process -= 1;
            }
            rx_mask <<= 1;   // for the current bit which is HIGH is stored as 0
        }
        else {
            rx_mask <<= bits_to_process - 1;   // if the bit is LOW, just move the mask
            rx_value |= rx_mask;  // store the LOW bit as 1
        }

        if rx_state > 7 {
            // check parity bit
            // let parity_bit_received = (rx_value >> 7) & 1 == 0;
            // let parity_bit_calculated = parity_bit(rx_value & 0x7F);
            // if parity_bit_received == parity_bit_calculated {
            //     char_to_buffer((rx_value & 0x7F) as char);
            // }
            // else {
            //     defmt::println!("Parity error, discarding byte");
            //     start_char();
            //     return;
            // }
            char_to_buffer((rx_value & 0x7F) as char);

            if gpio_state == false || !next_char_started {
                rx_state = WAITING_FOR_START_BIT;
            }
            else {
                start_char();
                return;
            }
        }
    }
    unsafe {
        RX_STATE = rx_state;
        RX_VALUE = rx_value;
        RX_MASK = rx_mask;
    }
}

pub fn probe_interrupt_handler(now: u32, gpio_state: bool) {

    let dt = now.wrapping_sub(unsafe { LAST_TICK });
    let bits_passed = num_bits_passed(dt);
    let mut rx_state = unsafe { RX_STATE };
    let mut rx_value = unsafe { RX_VALUE };
    let mut rx_mask = unsafe { RX_MASK };
    // defmt::println!("Interrupt! gpio_state: {}, LAST_TICK: {}, dt: {}, bits_passed: {}, rx_state: {}", gpio_state, unsafe { LAST_TICK }, dt, bits_passed, rx_state);
    unsafe { LAST_TICK = now; }

    match rx_state {
        WAITING_FOR_BREAK => {
            if gpio_state == true {
                defmt::println!("Break started!");
                rx_state = WAITING_FOR_MARK;
            }
        }
        WAITING_FOR_MARK => {
            if gpio_state == false {
                if dt > (SDI12_MARK_DURATION_US - SDI12_TIMING_TOLERANCE) as u32 {
                    defmt::println!("Valid marking condition detected, ready to receive data");
                    rx_state = WAITING_FOR_START_AFTER_BREAK;
                }
                else {
                    defmt::println!("Invalid marking condition, waiting for break again");
                    rx_state = WAITING_FOR_BREAK;
                }
            }
            else {
                rx_state = WAITING_FOR_BREAK;
            }
        }
        WAITING_FOR_START_AFTER_BREAK => {
            if gpio_state == true {
                // it is a start bit after valid marking condition
                if dt > (SDI12_MARK_DURATION_US - SDI12_TIMING_TOLERANCE) as u32 {
                    unsafe { RECEIVED_BREAK = true; }
                    start_char();
                    return;
                }
                else {
                    defmt::println!("Invalid marking condition, waiting for break again");
                    rx_state = WAITING_FOR_BREAK;
                }
            }
            else {
                rx_state = WAITING_FOR_BREAK;
            }
        }
        WAITING_FOR_START_BIT => {
            if gpio_state == true {
                // it is a start bit
                start_char();
                return;
            }
        }
        _ => {
            // Receiving bits for a byte
            let bits_left = 9 - rx_state;
            let next_char_started = bits_passed > bits_left as u16;
            let mut bits_to_process = if next_char_started { bits_left } else { bits_passed as u8 };
            rx_state += bits_to_process;

            if gpio_state == true {
                while bits_to_process > 0 {
                    rx_value |= rx_mask;      // all the LOW bits are stored as 1
                    rx_mask <<= 1;
                    bits_to_process -= 1;
                }
                rx_mask <<= 1;   // for the current bit which is HIGH is stored as 0
            }
            else {
                rx_mask <<= bits_to_process - 1;   // if the bit is LOW, just move the mask
                rx_value |= rx_mask;  // store the LOW bit as 1
            }

            if rx_state > 7 {
                // check parity bit
                // let parity_bit_received = (rx_value >> 7) & 1 == 0;
                // let parity_bit_calculated = parity_bit(rx_value & 0x7F);
                // if parity_bit_received == parity_bit_calculated {
                //     char_to_buffer((rx_value & 0x7F) as char);
                // }
                // else {
                //     defmt::println!("Parity error, discarding byte");
                //     start_char();
                //     return;
                // }
                char_to_buffer((rx_value & 0x7F) as char);
                if gpio_state == false || !next_char_started {
                    rx_state = WAITING_FOR_START_BIT;
                }
                else {
                    start_char();
                    return;
                }
            }
        }
    }

    unsafe {
        RX_STATE = rx_state;
        RX_VALUE = rx_value;
        RX_MASK = rx_mask;
    }
}
