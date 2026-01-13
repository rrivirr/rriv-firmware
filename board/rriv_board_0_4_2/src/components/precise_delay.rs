use core::arch::asm;

use embedded_hal::blocking::delay::DelayUs;
use stm32f1xx_hal::pac::DWT;

use crate::SYSCLK_MHZ;

static PER_MICROSEC: u16 = SYSCLK_MHZ as u16;

pub struct PreciseDelayUs {
}

impl PreciseDelayUs {
    pub fn new() -> Self {
        Self {  }
    }
}

impl DelayUs<u16> for PreciseDelayUs {
    fn delay_us(&mut self, us: u16) {
        unsafe {
            let real_cyc = (us * PER_MICROSEC) as u32 / 4;
            asm!(
                // Use local labels to avoid R_ARM_THM_JUMP8 relocations which fail on thumbv6m.
                "1:",
                "subs {}, #1",
                "bne 1b",
                inout(reg) real_cyc => _,
                options(nomem, nostack),
            );
        }
    }
}
