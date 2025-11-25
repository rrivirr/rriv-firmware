
use rtt_target::rprintln;
use stm32f1xx_hal::{afio::MAPR, pac::{self, NVIC, RCC, UART5}, prelude::*, rcc::{BusClock, Clocks}, serial::{Config, Parity, Serial, StopBits, WordLength}};

use crate::{interrupt, pin_groups};

use crate::pac::uart5 as uart_base;

pub fn setup_serialb(
    pins: pin_groups::SerialPins,
    // mapr: &mut MAPR, // no remapping for UART5
    uart: UART5,
    clocks: &Clocks,
){

    let config = Config::default()
                .baudrate(115200.bps());

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
            // Err(nb::Error::Other(err))
        } else {
            // Check if a byte is available
            if sr.rxne().bit_is_set() {
                // Read the received byte
                // Ok(
                let byte = usart.dr.read().dr().bits();
                rprintln!("uart5  rx byte: {}", byte);
                // )
            } else {
                // Err(nb::Error::WouldBlock)
            }
        }


//     //     // if let Some(ref mut rx) = USART_RX.borrow(cs).borrow_mut().deref_mut() {
//     //     //     if rx.is_rx_not_empty() {
//     //     //         if let Ok(c) = nb::block!(rx.read()) {
//     //     //             rprintln!("serial rx byte: {}", c);
//     //     //             // USART_UNREAD_MESSAGE = true;
//     //     //             // if USART_RECEIVE_INDEX < USART_RECEIVE_SIZE - 1 {
//     //     //             //     USART_RECEIVE[USART_RECEIVE_INDEX] = c;
//     //     //             //     USART_RECEIVE_INDEX = USART_RECEIVE_INDEX + 1;
//     //     //             // }

//     //     //             // let r = USART_RX_PROCESSOR.borrow(cs);
//     //     //             // if let Some(processor) = r.borrow_mut().deref_mut() {
//     //     //             //     processor.process_character(c.clone());
//     //     //             // }
//     //     //         }
//     //     //     }
//     //     // }
    })
}