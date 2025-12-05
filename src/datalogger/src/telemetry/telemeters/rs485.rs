use core::time;

use rriv_board::RRIVBoard;

use alloc::format;
use alloc::boxed::Box;


// pub struct Serial {
// }


// impl Serial {
//     pub fn run_loop_iteration(&mut self, board: &mut impl RRIVBoard) {
//         board.usart_send(format!("timestamp:{}", board.timestamp()).as_str());
//     }
// }

pub fn transmit(board: &mut impl RRIVBoard, payload: Box<[u8]> ) {
    board.get_sensor_driver_services().set_gpio_pin_mode(1, rriv_board::gpio::GpioMode::PushPullOutput);

    let _ = board.get_sensor_driver_services().write_gpio_pin(1, true);
    for i in 0..payload.len() {
        board.usart_send(format!("{}", payload[i]).as_str() );
    }
    let _ = board.get_sensor_driver_services().write_gpio_pin(1, false);
}