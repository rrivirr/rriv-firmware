#![cfg_attr(not(test), no_std)]
#![allow(clippy::empty_loop)]
#![feature(alloc_error_handler)]
#![no_main]

// extern crate panic_halt;

use core::prelude::rust_2024::*;
use cortex_m_rt::entry;
use rriv_board_0_4_2::{HSE_MHZ, PCLK_MHZ, SYSCLK_MHZ};
use rtt_target::{rtt_init_defmt};
use stm32f1xx_hal::{flash::FlashExt, pac::TIM3, timer::DelayMs};

pub mod prelude;

use stm32f1xx_hal::{pac, prelude::*};

extern crate rriv_board;

extern crate rriv_board_0_4_2;
use crate::rriv_board::RRIVBoard;

extern crate datalogger;
use datalogger::DataLogger;



#[entry]
fn main() -> ! {
    rtt_init_defmt!();
    prelude::init();

    let mut board = rriv_board_0_4_2::build();
    defmt::println!("board built");  board.delay_ms(1000);

    board.start(); // not needed, for debug only
    defmt::println!("board started");  board.delay_ms(1000);
    
    let mut datalogger = DataLogger::new();
    defmt::println!("datalogger built");  board.delay_ms(1000);
    
    datalogger.setup(&mut board);
    defmt::println!("datalogger set up");  board.delay_ms(1000);
    
    board.watchdog.feed(); // make sure we leave enough time for the panic handler
    
    loop {
        board.run_loop_iteration();
        datalogger.run_loop_iteration(&mut board);
    }
}


use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    defmt::println!("Panicked!");
    if let Some(location) = _info.location() {
        defmt::println!("at {}", location);
    }
    
    defmt::println!("with message: {}", defmt::Display2Format(&_info.message()));
    let device_peripherals = unsafe { pac::Peripherals::steal() };
    
    let rcc = device_peripherals.RCC.constrain();
    let mut flash = device_peripherals.FLASH.constrain();
    let clocks = rcc.cfgr
            .use_hse(HSE_MHZ.MHz())
            .sysclk(SYSCLK_MHZ.MHz())
            .pclk1(PCLK_MHZ.MHz())
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
    defmt::println!("send json panic");

    // we use format! here because we didn't find another good way yet.
    // rriv_board_0_4_2::write_panic_to_storage(format!("Panick: {} \n", _info.message().as_str().unwrap_or_default()).as_str());

    loop {}
}