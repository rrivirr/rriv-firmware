
use rtt_target::rprintln;
use stm32f1xx_hal::{afio::MAPR, gpio, pac::{self, NVIC, RCC, UART5}, prelude::*, rcc::{BusClock, Clocks}, serial::{Config, Parity, Serial, StopBits, WordLength}};

use crate::{interrupt, pin_groups};

use crate::pac::uart5 as uart_base;
use stm32f1xx_hal::rcc::{Enable, Reset};

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


