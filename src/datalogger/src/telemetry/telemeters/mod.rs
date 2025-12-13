use rriv_board::RRIVBoard;

use crate::drivers::resources::gpio::GpioRequest;

pub mod lorawan;
pub mod telemeter;
pub mod modbus;

pub const MAX_DATA_VALUES : usize = 16usize;


pub trait Telemeter {
    fn run_loop_iteration(&mut self, board: &mut dyn RRIVBoard);
    fn transmit(&mut self, board: &mut dyn RRIVBoard, values: &[i16; MAX_DATA_VALUES]);
    fn ready_to_transmit(&mut self, board: &mut dyn RRIVBoard) -> bool;
    fn process_events(&mut self, board: &mut dyn RRIVBoard);
    fn get_requested_gpios(&self) -> GpioRequest;
}