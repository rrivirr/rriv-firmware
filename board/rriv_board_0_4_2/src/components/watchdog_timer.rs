// A custom watchdog timer that fires an interrupt, and can be disabled during stop mode

use rtt_target::rprint;
use stm32f1xx_hal::{pac::TIM5, rcc::Clocks, timer::{CounterMs, Event}};
use stm32f1xx_hal::{
    time::MilliSeconds,
    timer::TimerExt
};

const WATCHDOG_PERIOD: u32 = 6;

pub struct WatchdogTimer {
    timer: CounterMs<TIM5>
}

impl WatchdogTimer {
    pub fn new(tim5: TIM5, clocks: &Clocks) -> WatchdogTimer {
        let timer: CounterMs<TIM5> = tim5.counter_ms(clocks);
        Self {
            timer
        }
    }

    pub fn start(&mut self){
        self.timer.start( MilliSeconds::secs(WATCHDOG_PERIOD)).unwrap();
        self.timer.listen(Event::Update);
    }

    pub fn feed(&mut self){
        self.timer.start(MilliSeconds::secs(WATCHDOG_PERIOD)).unwrap();
    }

    pub fn stop(&mut self){
        match self.timer.cancel() {
            Ok(_) => {},
            Err(err) => rprint!("{:?}", err),
        }
    }
}