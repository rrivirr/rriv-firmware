use rriv_board::{RRIVBoard, gpio};
use sdi12::*;
use core::fmt::{self, Write};

pub enum Sdi12Command {
    M,
    Mc(char),
    D(char),
}

struct CharBuffer<'a> {
    buffer: &'a mut [char],
    cursor: usize,
}

impl<'a> Write for CharBuffer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if self.cursor < self.buffer.len() {
                self.buffer[self.cursor] = c;
                self.cursor += 1;
            } else {
                // Return an error if we run out of space in the array
                return Err(fmt::Error); 
            }
        }
        Ok(())
    }
}

pub struct Sdi12Board <'a> {
    gpio: u8,
    board: &'a mut dyn RRIVBoard
}

impl<'a> Sdi12Board<'a> {
    pub fn new(gpio: u8, board: &'a mut dyn RRIVBoard) -> Self {
        Sdi12Board {
            gpio,
            board
        }
    }
}

impl<'a> BoardForSDI12 for Sdi12Board<'a> {
    fn write(&mut self, value: bool) {
        self.board.write_gpio_pin(self.gpio, value);
    }

    fn read(&mut self) -> bool {
        let result = self.board.read_gpio_pin(self.gpio);
        let value = match result {
            Ok(value) => value,
            Err(_) => false
        };
        value
    }

    fn delay_us(&mut self, us: u16) {
        self.board.delay_us(us);
    }

    fn pin_mode(&mut self, mode: gpio::GpioMode) {
        self.board.set_gpio_pin_mode(self.gpio, mode);
    }

    fn millis(&mut self) -> u32 {
        self.board.millis()
    }
}



pub fn setup(board: &mut dyn RRIVBoard) {
    // not implemented, does nothing.  
    // see notes below
}

pub struct Sdi12ByteProcessor {
    gpio: u8,
    awake: bool,
}

impl<'a> Sdi12ByteProcessor {
    pub fn new(gpio: u8) -> Sdi12ByteProcessor {
        Sdi12ByteProcessor {
            gpio: gpio,
            awake: false
        }
    }

    pub fn wake_up(&mut self, board: &mut dyn RRIVBoard) {
        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        self.awake = sdi12.receive_break();
    }

    pub fn is_awake(&mut self) -> bool {
        self.awake
    }

    pub fn sleep(&mut self) {
        self.awake = false;
    }

    pub fn take_message(&mut self, address: char, board: &mut dyn RRIVBoard) -> Result<Sdi12Command, &str> {
        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        let buffer = sdi12.read_command();
        if buffer[0] != address {
            self.sleep();
            defmt::println!("{} != {}", buffer[0], address);
            return Err("Address not matching");
        }
        let mode = match (buffer[1], buffer[2]) {
            ('M', '!') => Sdi12Command::M,
            ('M', d @ '0'..='9') => Sdi12Command::Mc(d),
            ('D', d @ '0'..='9') => Sdi12Command::D(d),
            _ => return Err("Invalid Command"),
        };

        board.delay_ms(10);

        Ok(mode)
    }

    pub fn send_MAck(&mut self, board: &mut dyn RRIVBoard, address: char, ttt: u32, n: u8) {
        let mut resp_buffer: [char; SDI12_BUFFER_SIZE] = ['0'; SDI12_BUFFER_SIZE];
        resp_buffer[0] = address;

        resp_buffer[1] = (b'0' + ((ttt / 100) % 10) as u8) as char; // Hundreds
        resp_buffer[2] = (b'0' + ((ttt / 10) % 10) as u8) as char;  // Tens
        resp_buffer[3] = (b'0' + (ttt % 10) as u8) as char;         // Ones

        resp_buffer[4] = (b'0' + n) as char;            

        resp_buffer[5] = '\r';
        resp_buffer[6] = '\n';
        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        sdi12.send_response(resp_buffer);
    }

    pub fn send_data(&mut self, board: &mut dyn RRIVBoard, address: char, data: [f64;9], n: u8) {
        let mut resp_buffer: [char; SDI12_BUFFER_SIZE] = ['\0'; SDI12_BUFFER_SIZE];

        let mut writer = CharBuffer {
            buffer: &mut resp_buffer,
            cursor: 0,
        };

        let _ = write!(writer, "{}", address);

        let mut i = 0;
        for &value in &data {
            let _ = write!(writer, "{:+.2}", value);
            i += 1;
            if i == n {
                break;
            }
        }

        // End with <CR><LF>
        let _ = write!(writer, "\r\n");
        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        sdi12.send_response(resp_buffer);
    }
}




