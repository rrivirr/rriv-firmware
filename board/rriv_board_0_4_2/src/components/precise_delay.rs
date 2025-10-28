use embedded_hal::blocking::delay::DelayUs;
use stm32f1xx_hal::pac::DWT;

static PER_MICROSEC: u16 = 72;

pub struct PreciseDelayUs {
    dwt: DWT
}

impl PreciseDelayUs {
    pub fn new(dwt: DWT) -> Self{
        Self {
            dwt
        }
    }
}

impl DelayUs<u16> for PreciseDelayUs {
    fn delay_us(&mut self, us: u16) {
        self.dwt.set_cycle_count(0);
        while DWT::cycle_count() < (us * PER_MICROSEC).into() {
            // Busy wait
        }
    }
}