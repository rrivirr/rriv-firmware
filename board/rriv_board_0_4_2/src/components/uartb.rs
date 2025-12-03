
use core::{cell::RefCell, ffi::FromBytesUntilNulError};

use cortex_m::interrupt::Mutex;
use rtt_target::rprintln;
use stm32f1xx_hal::{afio::MAPR, gpio, pac::{self, NVIC, RCC, UART5}, prelude::*, rcc::{BusClock, Clocks}, serial::{Config, Parity, Serial, StopBits, WordLength}};

use crate::{interrupt, pin_groups};

use crate::pac::uart5 as uart_base;
use stm32f1xx_hal::rcc::{Enable, Reset};

const BUFFER_SIZE: usize = 500;


pub struct SerialB
{
    cur: usize,
    end: usize,
    buffer: [u8; BUFFER_SIZE]
}

pub static mut SERIALB: Mutex<RefCell<SerialB>> = Mutex::new(RefCell::new(SerialB::default()));


impl SerialB {
    pub const fn default() -> Self {
        Self {
            cur: 0,
            end: BUFFER_SIZE - 1,
            buffer: [0; BUFFER_SIZE]

        }
    }

    pub fn process_byte(&mut self, byte : u8){
        if self.cur != self.end {
            self.buffer[self.cur] = byte;
            self.cur = self.cur + 1;
        }
    }

    pub fn take_message(&mut self, buffer: &mut [u8; 100]) -> bool {
        let delimeter = b'\r';
        // TODO: the next line require reordering
        if self.cur < self.end {
            //no wrapping
            if self.buffer[self.cur..self.end].contains(&delimeter) {
                let pos = match self.buffer[self.cur..self.end].into_iter().position( |&byte| byte == delimeter){
                    Some(pos) => pos,
                    None => { return false},
                };
                buffer.copy_from_slice(&self.buffer[self.cur..(self.cur+pos+1)]);
                return true;
            }
        }
        return false;
    }
}

pub fn setup_serialb(
    // pins: pin_groups::SerialPins,
    // mapr: &mut MAPR, // no remapping for UART5
    uart: UART5,
    clocks: &Clocks,
){

    let rcc = unsafe { &(*RCC::ptr()) };
        UART5::enable(rcc);
        UART5::reset(rcc);

    let config = Config::default()
                .baudrate(115200.bps())
                .wordlength_8bits()
                .parity_none()
                .stopbits(StopBits::STOP1);

    let rcc = unsafe { &(*RCC::ptr()) };
    
    // Configure baud rate
    let brr = UART5::clock(clocks).raw() / config.baudrate.0;
    assert!(brr >= 16, "impossible baud rate");
    uart.brr.write(|w| unsafe { w.bits(brr) });

    // Configure word
    uart.cr1.modify(|_r, w| {
        w.m().bit(match config.wordlength {
            WordLength::Bits8 => false,
            WordLength::Bits9 => true,
        });
        use crate::pac::uart5::cr1::PS_A;
        w.ps().variant(match config.parity {
            Parity::ParityOdd => PS_A::Odd,
            _ => PS_A::Even,
        });
        w.pce().bit(!matches!(config.parity, Parity::ParityNone));
        w
    });

    // Configure stop bits
    let stop_bits = match config.stopbits {
        StopBits::STOP1 => 0b00,
        StopBits::STOP0P5 => 0b01,
        StopBits::STOP2 => 0b10,
        StopBits::STOP1P5 => 0b11,
    };
    uart.cr2.modify(|_r: &uart_base::cr2::R, w| unsafe { w.stop().bits(stop_bits) });


    // UE: enable USART
    // TE: enable transceiver
    // RE: enable receiver
    uart.cr1.modify(|_r, w| {
        w.ue().set_bit();
        w.te().set_bit();
        w.re().set_bit();
        w
    });

    // listen on UART5
    unsafe { (*UART5::ptr()).cr1.modify(|_, w| w.rxneie().set_bit()) };

    unsafe {
        NVIC::unmask(pac::Interrupt::UART5);
    }


}




#[interrupt]
unsafe fn UART5() {
    cortex_m::interrupt::free(|cs| {

        // rx.is_rx_not_empty
        if ! unsafe { (*UART5::ptr()).sr.read().rxne().bit_is_set() } {
            return;
        }


        // rx.read
        let usart = unsafe { &*UART5::ptr() };
        let sr = usart.sr.read();

        // Check for any errors
        let err = if sr.pe().bit_is_set() {
            Some(stm32f1xx_hal::serial::Error::Parity)
        } else if sr.fe().bit_is_set() {
            Some(stm32f1xx_hal::serial::Error::Framing)
        } else if sr.ne().bit_is_set() {
            Some(stm32f1xx_hal::serial::Error::Noise)
        } else if sr.ore().bit_is_set() {
            Some(stm32f1xx_hal::serial::Error::Overrun)
        } else {
            None
        };

        if let Some(err) = err {
            // Some error occurred. In order to clear that error flag, you have to
            // do a read from the sr register followed by a read from the dr register.
            let _ = usart.sr.read();
            let _ = usart.dr.read();
            rprintln!("uart5 error {:?}", err);
            // Err(nb::Error::Other(err))
        } else {
            // Check if a byte is available
            if sr.rxne().bit_is_set() {
                // Read the received byte
                // Ok(
                let byte = usart.dr.read().dr().bits();
                rprintln!("uart5  rx byte: {}", byte as u8 as char); 
                cortex_m::interrupt::free(|cs| {
                    match SERIALB.borrow(cs).try_borrow_mut(){
                        Ok(mut serial) => {serial.process_byte(byte as u8);}
                        Err(_) => {}
                    }
                });
            } else {
                // Err(nb::Error::WouldBlock)
            }
        }   
    })
}
