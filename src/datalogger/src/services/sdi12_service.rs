use rriv_board::{RRIVBoard, gpio};
use sdi12::*;
use core::fmt::{self, Write};

pub enum Sdi12Command {
    M,
    Mc(char),
    D(char),
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
        self.board.run_loop_iteration(); // TODO: feed the watchdog, need a dedicated call for this though.
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
        if self.awake {
            board.usb_serial_send(format_args!("SDI12: received break\n"));
        }
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
            board.usb_serial_send(format_args!("SDI12: sleep\n"));
            self.sleep();
            defmt::println!("{} != {}", buffer[0], address);
            return Err("Address not matching");
        }
        
        board.usb_serial_send(format_args!("SDI12: received {}{}{}\n", buffer[0], buffer[1], buffer[2]));

        let mode = match (buffer[1], buffer[2]) {
            ('M', '!') => Sdi12Command::M,
            ('M', d @ '0'..='9') => Sdi12Command::Mc(d),
            ('D', d @ '0'..='9') => Sdi12Command::D(d),
            _ => return Err("Invalid Command"),
        };

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
        board.usb_serial_send(format_args!("SDI12: sent {}{}{}{}{}\n", resp_buffer[0], resp_buffer[1], resp_buffer[2], resp_buffer[3], resp_buffer[4])); // TODO: if self.watch
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
        board.usb_serial_send(format_args!("SDI12: sent {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\n", 
            resp_buffer[0], 
            resp_buffer[1], 
            resp_buffer[2], 
            resp_buffer[3], 
            resp_buffer[4], 
            resp_buffer[5], 
            resp_buffer[6], 
            resp_buffer[7], 
            resp_buffer[8],
            resp_buffer[9],
            resp_buffer[10], 
            resp_buffer[11], 
            resp_buffer[12], 
            resp_buffer[13], 
            resp_buffer[14], 
            resp_buffer[15], 
            resp_buffer[16], 
            resp_buffer[17], 
            resp_buffer[18],
            resp_buffer[19],
            resp_buffer[20], 
            resp_buffer[21], 
            resp_buffer[22], 
            resp_buffer[23], 
            resp_buffer[24], 
            resp_buffer[25], 
            resp_buffer[26], 
            resp_buffer[27], 
            resp_buffer[28],
            resp_buffer[29]
        )); // TODO: if self.watch

    }

    pub fn send_m_command(&mut self, board: &mut dyn RRIVBoard, address: char, id: char) -> SDI12_MResponse {
        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        
        let mut command : [char; SDI12_COMMAND_SIZE] = ['\0'; SDI12_COMMAND_SIZE];
        command[0] = address;
        command[1] = 'M';
        if id == '\0' {
            command[2] = '!';
        }
        else {
            command[2] = id;
            command[3] = '!';
        }
        sdi12.send_break();
        sdi12.send_command(command);
        defmt::println!("Sent 0M0!");

        let response = sdi12.read_response();

        board.usb_serial_send(format_args!("SDI12: sent {}{}{}{}\n", command[0], command[1], command[2], command[3])); // TODO: if self.watch
        board.usb_serial_send(format_args!("SDI12: received {}{}{}{}{}\n", response[0], response[1], response[2], response[3], response[4])); // TODO: if self.watch

        // parse the response
        // format: <address>tttn<CR><LF>
        let address_r = response[0];
        if address_r != address {
            // invalid response
            return SDI12_MResponse {
                address: '\0',
                ttt: 0,
                n: 0
            };
        }

        let ttt = &response[1..4];
        // convert ttt from ASCII to integer
        let mut result: u32 = 0; // Or u32, usize, etc.

        for &c in ttt {
            // to_digit(10) converts the char to a number from 0-9
            let digit = c.to_digit(10);
            if let Some(d) = digit {
                result = (result * 10) + d as u32;
            }
        }

        let n : u8 = response[4].to_digit(10).unwrap_or(0) as u8; // convert ASCII to integer

        // self.sdi12_board.delay_us(SDI12_GAP);
        
        SDI12_MResponse {
            address: address_r,
            ttt: result,
            n: n
        }
    }

    pub fn send_d0_command(&mut self, board: &mut dyn RRIVBoard, address: char, num_data: u8) -> SDI12_Dresponse {
        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        
        let mut command : [char; SDI12_COMMAND_SIZE] = ['\0'; SDI12_COMMAND_SIZE];
        command[0] = address;
        command[1] = 'D';
        command[2] = '0';
        command[3] = '!';
        let mut resp : SDI12_Dresponse = SDI12_Dresponse {
            address: '\0',
            data: [0.0; 9],
            count: 0,
            terminate: false,
            last_d_ind: 0
        };
        
        sdi12.send_break();
        sdi12.send_command(command);
        defmt::println!("Sent 0D0!");
        let response = sdi12.read_response();

        board.usb_serial_send(format_args!("SDI12: sent {}{}{}{}{}\n", command[0], command[1], command[2], command[3], command[4])); // TODO: if self.watch


        let my_board = Sdi12Board::new(self.gpio, board);
        let mut sdi12 = SDI12::new(my_board);
        
        // parse the response
        // format: <address><data><CR><LF>
        let address_r = response[0];
        if address_r != address {
            // invalid response
            return resp;
        }
        resp.address = address_r;
        let response = &response[1..SDI12_BUFFER_SIZE];
        (resp.data, resp.count) = sdi12.parse_data(response);
        
        if resp.count == num_data {
            resp.terminate = true;
        }

        board.usb_serial_send(format_args!("SDI12: received {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\n", 
            response[0], 
            response[1], 
            response[2], 
            response[3], 
            response[4], 
            response[5], 
            response[6], 
            response[7], 
            response[8],
            response[9],
            response[10], 
            response[11], 
            response[12], 
            response[13], 
            response[14], 
            response[15], 
            response[16], 
            response[17], 
            response[18],
            response[19],
            response[20], 
            response[21], 
            response[22], 
            response[23], 
            response[24], 
            response[25], 
            response[26], 
            response[27], 
            response[28],
            response[29]
        )); // TODO: if self.watch

        resp
    }
}




