use core::fmt::Write;

use rriv_board::RRIVBoard;
use serde_json::json;
use util::str_from_utf8;

use crate::drivers::resources::gpio::GpioRequest;
use crate::services::usart_service;
use alloc::string::{String,ToString};

#[derive(Clone, Copy)]
enum RakWireless3172Step {
    Begin = 0,
    StopJoinConfirm = 1,
    SetBand = 2,
    SetBandConfirm = 3,
    SetMask = 4,
    SetMaskConfirm = 5,
    StartJoin = 6,
    StartJoinConfirm = 7,
    CheckJoined = 8,
    Joined = 9,
    Undefined = 255,
}

impl RakWireless3172Step {
    fn from_integer(v: u8) -> Self {
        match v {
            0 => Self::Begin,
            1 => Self::StopJoinConfirm,
            2 => Self::SetBand,
            3 => Self::SetBandConfirm,
            4 => Self::SetMask,
            5 => Self::SetMaskConfirm,
            6 => Self::StartJoin,
            7 => Self::StartJoinConfirm,
            8 => Self::CheckJoined,
            9 => Self::Joined,
            _ => Self::Undefined,
        }
    }

    fn next(self) -> Self {
        Self::from_integer((self as u8) + 1)
    }

    fn status(&self) -> String {
        match self {
            RakWireless3172Step::Joined => {
                return String::from("Joined");
            }
            _ => {
                return String::from("Not Joined");
            }
        }
    }
}

pub struct RakWireless3172 {
    telemetry_step: RakWireless3172Step,
    usart_send_time: i64,
    last_transmission: i64,
    watch: bool,
}

impl RakWireless3172 {
    pub fn new() -> RakWireless3172 {
        return Self {
            telemetry_step: RakWireless3172Step::Begin,
            usart_send_time: 0,
            last_transmission: 0,
            watch: false,
        };
    }

    pub fn status(&self) -> String {
        self.telemetry_step.status()
    }

    pub fn set_watch(&mut self, watch: bool) {
        self.watch = watch;
    }

    fn send_and_increment_step(&mut self, board: &mut impl RRIVBoard, message: &str) {
        let prepared_message = format_args!("{}\r\n", message);
        usart_service::format_and_send(board, prepared_message);        
        self.usart_send_time = board.timestamp();
        self.telemetry_step = self.telemetry_step.next();
        defmt::println!("trying telemetry step {}", self.telemetry_step as u8);
    }

    fn check_ok_or_restart(&mut self, board: &mut impl RRIVBoard) {
        match usart_service::take_command(board) {
            Ok(message) => {
                let mut message = message;
                let message = util::str_from_utf8(&mut message);
                match message {
                    Ok(message) => match message.find("OK") {
                        Some(index) => {
                            if index == 0 {
                                self.telemetry_step = self.telemetry_step.next();
                                defmt::println!("trying telemetry step {}", self.telemetry_step as u8);
                                return;
                            } else {
                                board.usb_serial_send(format_args!("LoRaWAN: {}\n", message));
                                defmt::println!("telem not ok: {}", message);
                                self.telemetry_step = RakWireless3172Step::Begin;
                                return;
                            }
                        }
                        None => {
                            board.usb_serial_send(format_args!("LoRaWAN: {}\n", message));
                            defmt::println!("telem not ok: {}", message);
                            self.telemetry_step = RakWireless3172Step::Begin;
                            return;
                        }
                    },
                    Err(e) => {
                        defmt::println!("telem message not ok: {:?}", defmt::Debug2Format(&e));
                        self.telemetry_step = RakWireless3172Step::Begin; // bad message
                        return;
                    }
                }
            }
            Err(_) => {} // no command ready, check timeout
        }

        // need to check for a timeout here
        // defmt::println!("no message, checking timeout");
        if board.timestamp() - self.usart_send_time > 2 {
            defmt::println!("timed out, going to step 0");
            self.telemetry_step = RakWireless3172Step::Begin;
        }
    }

    // TODO: need a command recieved queue, just like for USB
    // AT_BUSY_ERROR
    // Restricted_Wait_158785
    fn check_joined(&mut self, board: &mut impl RRIVBoard) {
        while match usart_service::take_command(board) {
            Ok(message) => {
                // handle join
                let mut message = message;
                let message = str_from_utf8(&mut message);
                let message = message.unwrap_or("invalid message");

                if message.contains("+EVT") || message.contains("AT+") || message.contains("Restricted"){
                    if self.watch {
                        board.usb_serial_send(format_args!("LoRaWAN: {}\n", message))
                    }
                }

                match message.find("+EVT:JOINED") {
                    Some(index) => {
                        if index == 0 {
                            self.telemetry_step = self.telemetry_step.next();
                            defmt::println!("Joined!!");
                            return;
                        }
                        true
                    }
                    None => false,
                }
            }
            Err(_) => false,
        } {}

        // checking timeout if we didn't return above
        if board.timestamp() - self.usart_send_time > 120 {
            // could power cycle here, but consider effect on intADC
            self.telemetry_step = RakWireless3172Step::Begin;
        }
    }

    pub fn run_loop_iteration(&mut self, board: &mut impl RRIVBoard) {
        match self.telemetry_step {
            RakWireless3172Step::Begin => {
                defmt::println!("trying telemetry step {}", self.telemetry_step as u8);
                self.send_and_increment_step(board, "AT+JOIN=0");
            }
            RakWireless3172Step::StopJoinConfirm => {
                self.check_ok_or_restart(board);
            }
            RakWireless3172Step::SetBand => {
                self.send_and_increment_step(board, "AT+BAND=5");
            }
            RakWireless3172Step::SetBandConfirm => {
                self.check_ok_or_restart(board);
            }
            RakWireless3172Step::SetMask => {
                self.send_and_increment_step(board, "AT+MASK=0002");
            }
            RakWireless3172Step::SetMaskConfirm => {
                self.check_ok_or_restart(board);
            }
            RakWireless3172Step::StartJoin => {
                self.send_and_increment_step(board, "AT+JOIN=1:0:15:100");
            }
            RakWireless3172Step::StartJoinConfirm => {
                self.check_ok_or_restart(board);
            }
            RakWireless3172Step::CheckJoined => {
                self.check_joined(board);
            }
            _ => {}
        }

        // defmt::println!("done setting up lorawan")
    }

    pub fn transmit(&mut self, board: &mut impl RRIVBoard, payload: &[u8]) {
        // AT+SEND=14:696E746572727570743

        let mut s = String::with_capacity(payload.len() * 2);
        for byte in payload {
            match write!(&mut s, "{:02X}", byte) {
                Ok(_) => {},
                Err(err) => defmt::println!("{}", defmt::Debug2Format(&err)),
            }
        }

        let args = format_args!("AT+SEND={}:{}\r\n", payload.len(), s.as_str()); 
        usart_service::format_and_send(board, args);
        self.last_transmission = board.timestamp();
    }

    pub fn ready_to_transmit(&mut self, board: &mut impl RRIVBoard) -> bool {
        if self.status() != "Joined" {
            return false;
        }

        if board.timestamp() < self.last_transmission + 10 {
            false
        } else {
            true
        }
    }

    // return binary command request here, if we got one.
    pub fn process_events(&mut self, board: &mut impl RRIVBoard) {
        
        match self.telemetry_step {
            RakWireless3172Step::Joined => {},
            _ => return
        }
        
        // TODO: event processing could be combined for all EVT and OK and AT
        // this probably means adding a local queue for retreived messages
        while match usart_service::take_command(board) {
            Ok(message) => {
                let mut message = message;
                let message = str_from_utf8(&mut message);
                let message = message.unwrap_or("invalid message");
                defmt::println!("lorawan: {}", message);

                // handle other events
                if message.contains("+EVT") || message.contains("AT") || message.contains("Restricted"){
                    if self.watch {
                        board.usb_serial_send(format_args!("LoRaWAN: {}\n", message))
                    }
                    if message.starts_with("AT_NO_NETWORK_JOINED") {
                        self.telemetry_step = RakWireless3172Step::Begin;
                    } else if message.starts_with("AT_BUSY_ERROR") {
                        // duty cycle or other busyness
                    } else {

                    }

                }
                true
            }
            Err(_) => false,
        } {}
    }

    pub fn get_identity(&mut self, board: &mut impl RRIVBoard) -> Result<String,()>{
        // get Dev EUI and Join EUI sychronously from the board

        let mut dev_eui: String = String::new(); // TODO: consider handling this in more pure no_std
        let mut join_eui: String = String::new();

        let message = "AT+DEVEUI=?";
        let prepared_message = format_args!("{}\r\n", message);
        usart_service::format_and_send(board, prepared_message);
        board.delay_ms(1000); // let the chip respond
        while match usart_service::take_command(board) { // because other async stuff could happen in the meantime
            Ok(message) => {
                let mut message = message;
                let message = str_from_utf8(&mut message);
                let message = message.unwrap_or("invalid message");
                defmt::println!("lorawan2: {}", message);

                let mut continuing: bool = true;
                // handle the response we are looking for
                if message.contains("AT+DEVEUI="){
                    match message.find("=") {
                        Some(index) => {
                            let index: usize = index + 1;
                            dev_eui = message[index..message.len()].to_string();
                            continuing = false;
                        },
                        None => {}, // continue
                    }
                   
                }
                continuing
            }
            Err(_) => return Err(()), // an empty receiving buffer will trigger here
        } {}

        let message = "AT+APPEUI=?";
        let prepared_message = format_args!("{}\r\n", message);
        usart_service::format_and_send(board, prepared_message);
        board.delay_ms(1000); // let the chip respond
        while match usart_service::take_command(board) { // because other async stuff could happen in the meantime
            Ok(message) => {
                let mut message = message;
                let message = str_from_utf8(&mut message);
                let message = message.unwrap_or("invalid message");
                defmt::println!("lorawan2: {}", message);

                let mut continuing: bool = true;
                // handle the response we are looking for
                if message.contains("AT+APPEUI="){
                    match message.find("=") {
                        Some(index) => {
                            let index: usize = index + 1;
                            join_eui = message[index..message.len()].to_string();
                            continuing = false;
                        },
                        None => return Err(()),
                    }
                   
                }
                continuing
            }
            Err(_) => return Err(()),
        } {}

        // TODO: find a way to not put the json serialization directly in this file
        let identity = json!(
            {
                "dev_eui" : dev_eui,
                "join_eui" : join_eui
            }
        ).to_string();

        Ok(identity)

    }

    pub fn get_requested_gpios(&self) -> GpioRequest {
        let mut request = GpioRequest::none();
        request.use_usart();
        return request;
    }
}
