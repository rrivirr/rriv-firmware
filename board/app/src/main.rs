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

extern crate datalogger;
use datalogger::DataLogger;

use rtt_target::rprintln;

extern crate alloc;
use alloc::format;


#[entry]
fn main() -> ! {
    rtt_init_print!();
    prelude::init();

    let mut board = rriv_board_0_4_2::build();
    board.start(); // not needed, for debug only
    let mut datalogger = DataLogger::new();
    datalogger.setup(&mut board);
    board.watchdog.feed(); // make sure we leave enough time for the panic handler
    loop {
        board.run_loop_iteration();
        datalogger.run_loop_iteration(&mut board);
    }
}


use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    if let Some(location) = _info.location() {
        rprintln!("at {}", location);
    }
    
    rprintln!("with message: {}", _info.message());
    let device_peripherals = unsafe { pac::Peripherals::steal() };
    
    let rcc = device_peripherals.RCC.constrain();
    let mut flash = device_peripherals.FLASH.constrain();
    let clocks = rcc.cfgr
            .use_hse(8.MHz())
            .sysclk(48.MHz())
            // .pclk1(36.MHz())
            // .adcclk(14.MHz())
            .freeze(&mut flash.acr);

    let mut delay: DelayMs<TIM3> = device_peripherals.TIM3.delay(&clocks);
    // we avoid using format! here because we don't want to do dynamic memory in panic handler
    rriv_board_0_4_2::usb_serial_send("{\"status\":\"panic\",\"message\":\"", &mut delay);
    // if let Some(location) = _info.location() {
    //     rriv_board_0_4_2::usb_serial_send(" at ", &mut delay);
    //     rriv_board_0_4_2::usb_serial_send(location., &mut delay);
    // }
    rriv_board_0_4_2::usb_serial_send(_info.message().as_str().unwrap_or_default(), &mut delay);
    rriv_board_0_4_2::usb_serial_send("\"}\n", &mut delay);
    rprintln!("send json panic");

    // we use format! here because we didn't find another good way yet.
    rriv_board_0_4_2::write_panic_to_storage(format!("Panick: {} \n", _info.message().as_str().unwrap_or_default()).as_str());

    loop {}
}