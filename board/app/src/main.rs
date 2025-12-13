#![cfg_attr(not(test), no_std)]
#![allow(clippy::empty_loop)]
#![feature(alloc_error_handler)]
#![no_main]

// extern crate panic_halt;

use core::prelude::rust_2024::*;
use cortex_m_rt::entry;
use rtt_target::{rtt_init_print};
use stm32f1xx_hal::{flash::FlashExt, pac::{self, TIM3}, time::{MicroSeconds, MilliSeconds}, timer::{DelayMs, DelayUs}};

pub mod prelude;

use stm32f1xx_hal::prelude::*;

extern crate rriv_board;

extern crate rriv_board_0_4_2;
use crate::rriv_board::RRIVBoard;

use rtt_target::rprintln;

extern crate alloc;
use alloc::format;


#[entry]
fn main() -> ! {
    rtt_init_print!();
    prelude::init();

    let mut board = rriv_board_0_4_2::build();
    board.start(); // for this build, just clears the eeprom
    loop {    
        board.delay.delay_ms(1000_u32);
    }
}


use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    if let Some(location) = _info.location() {
        rprintln!("at {}", location);
    }

    loop {}
}