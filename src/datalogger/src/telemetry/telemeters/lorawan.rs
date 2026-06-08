use core::fmt::Write;

use rriv_board::RRIVBoard;
use serde_json::json;
use util::str_from_utf8;

use crate::telemetry::codecs::naive_codec;
use crate::telemetry::telemeters::lorawan::LoRaWANTelemetryStatus::Joined;
use crate::{drivers::resources::gpio::GpioRequest, telemetry::telemeters::Telemeter};
use crate::services::usart_service;
use alloc::string::{String,ToString};

const JOIN_TIMEOUT:i64 = 30; // if we don't join within 30s, give up
const JOIN_BACKOFF:i64 = 60*60; // if we time out, wait for 1 hour before trying again

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
    QueryResumableJoin = 10,
    CheckResumableJoin = 11,
    Undefined = 255,
}

pub enum LoRaWANTelemetryStatus {
    Joined,
    NotJoined,
    TimedOut
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
            10 => Self::QueryResumableJoin,
            11 => Self::CheckResumableJoin,
            _ => Self::Undefined,
        }
    }

    fn next(self) -> Self {
        Self::from_integer((self as u8) + 1)
    }

    fn status(&self) -> LoRaWANTelemetryStatus {
        match self {
            RakWireless3172Step::Joined => {
                return LoRaWANTelemetryStatus::Joined;
            }
            _ => {
                return LoRaWANTelemetryStatus::NotJoined;
            }
        }
    }
}

pub struct RakWireless3172 {
    telemetry_step: RakWireless3172Step,
    usart_send_time: u32,
    last_transmission: u32,
    watch: bool,
    started_at: i64,
    timed_out_at: i64,
}

impl RakWireless3172 {
    pub fn new() -> RakWireless3172 {
        return Self {
            telemetry_step: RakWireless3172Step::Begin,
            usart_send_time: 0,
            last_transmission: 0,
            watch: false,
            started_at: 0,
            timed_out_at: -1
        };
    }
    
    pub fn resume() -> RakWireless3172 {
       return Self {
            telemetry_step: RakWireless3172Step::QueryResumableJoin,
            usart_send_time: 0,
            last_transmission: 0,
            watch: false,
            started_at: 0,
            timed_out_at: -1
        }; 
    }

    pub fn start(&mut self, board: &mut dyn RRIVBoard) {
        self.started_at = board.epoch_timestamp();
        // TODO: we need to get the last time out so we can span standby/restart
    }

    pub fn status(&self) -> LoRaWANTelemetryStatus {
        return match self.telemetry_step.status() {
            LoRaWANTelemetryStatus::Joined => {
                LoRaWANTelemetryStatus::Joined
            },
            _ => {
                if self.timed_out_at != -1 {
                    LoRaWANTelemetryStatus::TimedOut
                } else {
                    LoRaWANTelemetryStatus::NotJoined
                }
            }
        }
    }

    pub fn set_watch(&mut self, watch: bool) {
        self.watch = watch;
    }

    fn send_and_increment_step(&mut self, board: &mut dyn RRIVBoard, message: &str) {
        let prepared_message = format_args!("{}\r\n", message);
        usart_service::format_and_send(board, prepared_message);        
        self.usart_send_time = board.seconds();
        self.telemetry_step = self.telemetry_step.next();
        defmt::println!("trying telemetry step {}", self.telemetry_step as u8);
    }

    fn check_ok_or_restart(&mut self, board: &mut dyn RRIVBoard) {
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
                                // board.usb_serial_send(format_args!("LoRaWAN: {}\n", message));
                                defmt::println!("telem not ok: {}", message);
                                self.telemetry_step = RakWireless3172Step::Begin;
                                return;
                            }
                        }
                        None => {
                            // board.usb_serial_send(format_args!("LoRaWAN: {}\n", message));
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
        if (board.seconds() as i32  - self.usart_send_time as i32).rem_euclid(60) > 2 {
            defmt::println!("timed out, going to step 0");
            self.telemetry_step = RakWireless3172Step::Begin;
        }
    }

   fn join_status_good_or_restart(&mut self, board: &mut dyn RRIVBoard) {
        match usart_service::take_command(board) {
            Ok(message) => {
                let mut message = message;
                let message = util::str_from_utf8(&mut message);
                match message {
                    Ok(message) => match message.find("AT+NJS=1") {
                        Some(index) => {
                            if index == 0 {
                                defmt::println!("already joined!");
                                self.telemetry_step = RakWireless3172Step::Joined;
                                return;
                            } else {
                                // board.usb_serial_send(format_args!("LoRaWAN: {}\n", message));
                                defmt::println!("telem not ok: {}", message);
                                self.telemetry_step = RakWireless3172Step::Begin;
                                return;
                            }
                        }
                        None => {
                            // board.usb_serial_send(format_args!("LoRaWAN: {}\n", message));
                            defmt::println!("not joined: {}", message);
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
        if (board.seconds() as i32  - self.usart_send_time as i32).rem_euclid(60) > 2 {
            defmt::println!("timed out, going to step 0");
            self.telemetry_step = RakWireless3172Step::Begin;
        }
    }

    // TODO: need a command recieved queue, just like for USB
    // AT_BUSY_ERROR
    // Restricted_Wait_158785
    fn check_joined(&mut self, board: &mut dyn RRIVBoard) {
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
        if (board.seconds() as i32 - self.usart_send_time as i32).rem_euclid(60) > 120 {
            // could power cycle here, but consider effect on intADC
            self.telemetry_step = RakWireless3172Step::Begin;
        }
    }

     pub fn get_identity(&mut self, board: &mut dyn RRIVBoard) -> Result<String,()>{
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
}

impl Telemeter for RakWireless3172 {
    fn run_loop_iteration(&mut self, board: &mut dyn RRIVBoard) {

        // check if we are timed out, and do not run loop logic if so

        match self.telemetry_step {
            RakWireless3172Step::Joined => {
                // ok
            },
            _ => {
                let now = board.epoch_timestamp();
                if self.timed_out_at == -1_i64 { // we are not timed out yet
                    // check for timeout
                    if now - self.started_at > JOIN_TIMEOUT { // try to join for 30s, then time out
                        self.timed_out_at = now;
                        // TODO: need to perist this so we can get it post-reset
                    }
                    // else no timeout, still good to keep trying join

                } else { // we are timed out
                    // check if backoff is elapsed
                    if now - self.timed_out_at > JOIN_BACKOFF { // time out for an hour before retry join
                        self.timed_out_at = -1;
                        self.started_at = now;
                    } else {
                        return; // skip the iteration, we are timed out
                    }
                }

            }
        }


        match self.telemetry_step {
            RakWireless3172Step::Begin => {
                defmt::println!("trying telemetry step {}", self.telemetry_step as u8);
                let mut drained = false;
                while drained == false {
                    drained = match usart_service::take_command(board) {
                        Ok(_) => false,
                        Err(_) => true,
                    }
                }
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
            RakWireless3172Step::Joined => {
                // do nothing
            },
            RakWireless3172Step::QueryResumableJoin => {
                self.send_and_increment_step(board, "AT+NJS=?");
            },
            RakWireless3172Step::CheckResumableJoin => {
                self.join_status_good_or_restart(board);
            }
            _ => {}
        }

        // defmt::println!("done setting up lorawan")
    }


    fn transmit(&mut self, board: &mut dyn RRIVBoard, values: &[i16]) {
        // AT+SEND=14:696E746572727570743


        let mut bytes: [u8; naive_codec::MAX_BYTES] = [0; naive_codec::MAX_BYTES];
        let size = naive_codec::encode(board.epoch_timestamp(), &values, &mut bytes);
       

        let mut s = String::with_capacity(size * 2);
        for byte in &bytes[0..size] {
            match write!(&mut s, "{:02X}", byte) {
                Ok(_) => {},
                Err(err) => defmt::println!("{}", defmt::Debug2Format(&err)),
            }
        }

        let args = format_args!("AT+SEND={}:{}\r\n", size, s.as_str()); 
        usart_service::format_and_send(board, args);
        self.last_transmission = board.seconds();
    }

    fn ready_to_transmit(&mut self, board: &mut dyn RRIVBoard) -> bool {

        if let Joined = self.status() {
            return false;
        }

        if (board.seconds() as i32 - self.last_transmission as i32).rem_euclid(60) < 10 {
            false
        } else {
            true
        }
    }

    // return binary command request here, if we got one.
    fn process_events(&mut self, board: &mut dyn RRIVBoard) {
        
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
                        // LoRaWAN: +EVT:RX_1:-60:11:UNICAST:10:41
                        // 41 is the payload here
                        // we need to pass back to the datalogger, and let the datalogger apply the change
                    }

                }
                true
            }
            Err(_) => false,
        } {}
    }

    fn get_requested_gpios(&self) -> GpioRequest {
        let mut request = GpioRequest::none();
        request.use_usart();
        return request;
    }
}
