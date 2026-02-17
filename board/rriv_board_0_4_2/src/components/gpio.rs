#[macro_export]
macro_rules! write_gpio {
    ($gpio:ident, $value:expr) => {
        if $value {
            let _ = $gpio.set_high();
        } else {
            let _ = $gpio.set_low();
        }
    };
}

#[macro_export]
macro_rules! read_pin {
    ($pin: ident) => {
        match $pin.is_high() {
            Ok(is_high) => Ok(is_high),
            Err(err) => {
                defmt::println!("{:?}", defmt::Debug2Format(&err));
                Err(())
            }
        }
    }
}

#[macro_export]
macro_rules! set_pin_mode {
    ($pin: ident, $cr: ident, $mode: ident) => {
        match $mode {
            rriv_board::gpio::GpioMode::FloatingInput => {
                $pin.make_floating_input($cr);
            }
            rriv_board::gpio::GpioMode::PullUpInput => {
                $pin.make_pull_up_input($cr);
            }
            rriv_board::gpio::GpioMode::PullDownInput => {
                $pin.make_pull_down_input($cr);
            }
            rriv_board::gpio::GpioMode::PushPullOutput => {
                $pin.make_push_pull_output($cr);
            }
            rriv_board::gpio::GpioMode::OpenDrainOutput => {
                $pin.make_open_drain_output($cr);
            }
            rriv_board::gpio::GpioMode::None => todo!(),
        }
    };
}

pub(crate) use write_gpio;
pub(crate) use read_pin;
pub(crate) use set_pin_mode;