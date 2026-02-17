#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use i2c_hung_fix::try_unhang_i2c;
use one_wire_bus::crc::crc8;
use rriv_board::hardware_error::{self, HardwareError};
use stm32f1xx_hal::time::{MilliSeconds, ms};
use stm32f1xx_hal::timer::{Ch, Channel, CounterUs, PwmHz, Tim4NoRemap};

use core::fmt::{self, Display};
use core::mem;
use core::{
    cell::RefCell,
    default::Default,
    ops::DerefMut,
    option::Option::{self, *},
    result::Result::*,
};
use cortex_m::{
    asm::{delay, dmb, dsb},
    interrupt::{CriticalSection, Mutex},
    peripheral::NVIC,
};

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32f1xx_hal::flash::ACR;
use stm32f1xx_hal::gpio::Pin;
use stm32f1xx_hal::pac::{DWT, I2C1, I2C2, TIM2, TIM4, TIM5, USART2, USB};
use stm32f1xx_hal::serial::StopBits;
use stm32f1xx_hal::spi::Spi;
use stm32f1xx_hal::{
    afio::MAPR,
    gpio::Dynamic,
    pac::TIM3,
    watchdog::IndependentWatchdog,
};

use stm32f1xx_hal::rcc::{Clocks, CFGR};
use stm32f1xx_hal::{
    gpio::{self, OpenDrain, Output},
    i2c::{BlockingI2c, Mode},
    pac,
    pac::interrupt,
    prelude::*,
    serial::{Config, Rx, Serial as Hal_Serial, Tx},
    timer::delay::*,
};

use stm32f1xx_hal::usb::{Peripheral, UsbBus, UsbBusType};
use usb_device::{bus::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use rriv_board::{
    EEPROM_TOTAL_SENSOR_SLOTS, RRIVBoard, RXProcessor, SerialRxPeripheral
};

use ds323x::{DateTimeAccess, Ds323x, NaiveDateTime};
use stm32f1xx_hal::rtc::Rtc;

use one_wire_bus::{crc::check_crc8, Address, OneWire, SearchState};

mod components;
use components::*;
// use components::gpio::read_pin;

mod pins;
use pins::{GpioCr, Pins};

mod pin_groups;
use pin_groups::*;

type RedLed = gpio::Pin<'A', 9, Output<OpenDrain>>;

pub const HSE_MHZ: u32 = 8;
pub const SYSCLK_MHZ: u32 = 48;
pub const PCLK_MHZ: u32 = 24;

static WAKE_LED: Mutex<RefCell<Option<RedLed>>> = Mutex::new(RefCell::new(None));
static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;
static mut USB_SERIAL: Option<usbd_serial::SerialPort<UsbBusType>> = None;
static mut USB_DEVICE: Option<UsbDevice<UsbBusType>> = None;

static USART_RX: Mutex<RefCell<Option<Rx<pac::USART2>>>> = Mutex::new(RefCell::new(None));
static USART_TX: Mutex<RefCell<Option<Tx<pac::USART2>>>> = Mutex::new(RefCell::new(None));

static RX_PROCESSOR: Mutex<RefCell<Option<Box<&mut dyn RXProcessor>>>> = Mutex::new(RefCell::new(None));
static USART2_RX_PROCESSOR: Mutex<RefCell<Option<Box<&mut dyn RXProcessor>>>> =
    Mutex::new(RefCell::new(None));
static UART5_RX_PROCESSOR: Mutex<RefCell<Option<Box<&mut dyn RXProcessor>>>> =
    Mutex::new(RefCell::new(None));

#[repr(C)]
pub struct Usart {
    tx: &'static Mutex<RefCell<Option<Tx<pac::USART2>>>>,
}

// type aliases to make things tenable
type BoardI2c1 = BlockingI2c<I2C1, (pin_groups::I2c1Scl, pin_groups::I2c1Sda)>;
type BoardI2c2 = BlockingI2c<I2C2, (pin_groups::I2c2Scl, pin_groups::I2c2Sda)>;

pub struct Board {
    pub uid: [u8; 12],
    pub delay: DelayUs<TIM3>,
    pub precise_delay: PreciseDelayUs,
    // // pub power_control: PowerControl,
    pub gpio: DynamicGpioPins,
    pub gpio_cr: GpioCr,
    pub internal_adc: InternalAdc,
    pub external_adc: ExternalAdc,
    pub battery_level: BatteryLevel,
    pub rgb_led: RgbLed,
    pub oscillator_control: OscillatorControl,
    pub i2c1: Option<BoardI2c1>,
    pub i2c2: BoardI2c2,
    pub internal_rtc: Rtc,
    pub storage: Option<Storage>,
    pub debug: bool,
    pub file_epoch: i64,
    pub one_wire_bus: Option<OneWire<OneWirePin<Pin<'C', 0, Dynamic>>>>,
    one_wire_search_state: Option<SearchState>,
    pub watchdog: IndependentWatchdog,
    pub counter: CounterUs<TIM5>,
    pub hardware_errors: [HardwareError; 5]
    pub clocks: Clocks,
    pub pwm: Option<Box<PwmHz<TIM4, Tim4NoRemap, Ch<2>, Pin<'B', 8, gpio::Alternate<OpenDrain>>>>>,
}

impl Board {
    pub fn start(&mut self) {
        defmt::println!("starting board");
        // self.power_control.cycle_3v(&mut self.delay);

        let timestamp: i64 = rriv_board::RRIVBoard::epoch_timestamp(self);
        if let Some(ref mut storage) = &mut self.storage {
            storage.create_file(timestamp);
        }

        // setting the pin for receiving telemetry on UART5
        // this crashes the mcu hard, maybe only if something isn't plugged in
        // defmt::println!("set up pin 2"); rriv_board::RRIVBoard::delay_ms(self, 1000);
        // self.get_sensor_driver_services().set_gpio_pin_mode(2, rriv_board::gpio::GpioMode::PushPullOutput);
        // self.get_sensor_driver_services().write_gpio_pin(2, false);
        // defmt::println!("pin 2 set up"); rriv_board::RRIVBoard::delay_ms(self, 1000);
        defmt::println!("board started");

    }

    pub fn sleep_mcu(&mut self) {
        // TODO: sleep mode won't work with independent watch dog, unless we can stop it.
        // EDIT: there is not alternative source for indep watchdog, it's always HSI
        // EDIT: therefore field mode must restart with indep watchdog disabled

        self.internal_rtc.set_alarm(5000); // 5 seconds?
        self.internal_rtc.listen_alarm();
        defmt::println!("will sleep");

        // disable interrupts
        NVIC::mask(pac::Interrupt::USB_HP_CAN_TX);
        NVIC::mask(pac::Interrupt::USB_LP_CAN_RX0);
        NVIC::mask(pac::Interrupt::USART2);

        unsafe { NVIC::unmask(pac::Interrupt::RTCALARM) };
        cortex_m::asm::dsb();

        let mut core_peripherals: pac::CorePeripherals = unsafe { cortex_m::Peripherals::steal() };
        core_peripherals.SYST.disable_interrupt();

        cortex_m::asm::wfi();

        core_peripherals.SYST.enable_interrupt();

        cortex_m::asm::isb();

        // re-enable interrupts
        unsafe { NVIC::unmask(pac::Interrupt::USB_HP_CAN_TX) };
        unsafe { NVIC::unmask(pac::Interrupt::USB_LP_CAN_RX0) };
        unsafe { NVIC::unmask(pac::Interrupt::USART2) };

        defmt::println!("woke from sleep");
    }

    pub fn enter_stop_mode(&mut self) {
        //           debug("setting up EXTI");
        //   *bb_perip(&EXTI_BASE->IMR, EXTI_RTC_ALARM_BIT) = 1;
        // 	*bb_perip(&EXTI_BASE->RTSR, EXTI_RTC_ALARM_BIT) = 1;

        // in addition to the RTCALARM interrupt, the rtc must route through EXTI to wake the MCU up from stop mode.
        let device_peripherals: pac::Peripherals = unsafe { pac::Peripherals::steal() };
        device_peripherals.EXTI.imr.write(
            |w| w.mr17().set_bit(), // interrupt mask bit 17 enables RTC EXTI
        );
        device_peripherals.EXTI.rtsr.write(
            |w| w.tr17().set_bit(), // rising trigger bit 17 enables RTC EXTI
        );

        // clocks
        // steal and use raw the same function sin cfgr to switch to hsi and wait for stabilization
        // and the same thing to switch back.
        // let clocks = cfgr
        //     .use_hse(8.MHz())
        //     .sysclk(48.MHz())
        //     .pclk1(24.MHz())
        //     .adcclk(14.MHz())
        //     .freeze(flash_acr);
    }

    fn disable_interrupts(&self) {
        // disable interrupts
        cortex_m::interrupt::disable();
    }

    fn enable_interrupts(&self) {
        // If the interrupts were active before our `disable` call, then re-enable
        // them. Otherwise, keep them disabled
        let primask = cortex_m::register::primask::read();
        if primask.is_active() {
            unsafe { cortex_m::interrupt::enable() }
        }
    }

    fn do_critical_section<T, F>(&self, f: F) -> T
    where
        F: Fn() -> T,
    {
        cortex_m::interrupt::free(|_cs| f())
    }

    fn add_hardware_error(&mut self, hardware_error: HardwareError){
        add_hardware_error(&mut self.hardware_errors, hardware_error);
    }
}

fn add_hardware_error(hardware_errors: &mut [HardwareError; 5], hardware_error: HardwareError){
    for i in 0..hardware_errors.len() {
        match hardware_errors[i] {
            HardwareError::None => {
                hardware_errors[i] = hardware_error;
                break;
            },
            _ => { // do nothing 
            }

        }
    }
}

impl RRIVBoard for Board {
    fn run_loop_iteration(&mut self) {
        self.watchdog.feed();

        self.file_epoch = self.epoch_timestamp();
    }

    fn set_serial_rx_processor(&mut self, peripheral: SerialRxPeripheral,  processor: Box<&'static mut dyn RXProcessor>) {
        cortex_m::interrupt::free(|cs| {

            let mut global_rx_binding  = match peripheral {
                SerialRxPeripheral::CommandSerial => {
                    RX_PROCESSOR.borrow(cs).borrow_mut()
                },
                SerialRxPeripheral::SerialPeripheral1 => {
                    USART2_RX_PROCESSOR.borrow(cs).borrow_mut()
                }
                SerialRxPeripheral::SerialPeripheral2 => {
                    UART5_RX_PROCESSOR.borrow(cs).borrow_mut()
                }
            };
                            
            *global_rx_binding = Some(processor);

        });
    }

    fn critical_section(&self, f: fn())
    {
        cortex_m::interrupt::free(|_cs| f())
    }

    // // use this to talk out on serial to other UART modules, RS 485, etc
    fn usart_send(&mut self, bytes: &[u8]) {

        cortex_m::interrupt::free(|cs| {
            // USART
            for byte in bytes.iter() {
                // defmt::println!("char {}", char);
                let t: &RefCell<Option<Tx<USART2>>> = USART_TX.borrow(cs);
                if let Some(tx) = t.borrow_mut().deref_mut() {
                    _ = nb::block!(tx.write(byte.clone()));
                }
            }

        });

        rriv_board::RRIVBoard::delay_ms(self, 2);
        
    }

    fn usb_serial_send(&mut self, arg: fmt::Arguments) { // TODO: ok so the formatter doesn't below in the board level, it can go into a util in the datalogger app level
        let mut buf = [0u8; 500];
        match format_no_std::show(
            &mut buf,
            arg
        ) {
            Ok(message) => {
                usb_serial_send(message, &mut self.delay);
                defmt::println!("{}", message); // TODO: this uses format!
            }
            Err(e) => {
                defmt::println!("format error {}", defmt::Debug2Format(&e));
            },
        }
    }

    // outputs to serial (which also echos to rtt)
    fn serial_debug(&mut self, args: fmt::Arguments) {

        let mut buf = [0u8; 64];
        match format_no_std::show(
            &mut buf,
            args
        ) {
            Ok(string) => {
                if self.debug {
                    rriv_board::RRIVBoard::usb_serial_send(self, format_args!("{{\"debug\":\"{}\"}}\n", string));
                } else {
                    defmt::println!("{}", string);
                }
            }
            Err(_) => {},
        }

        
    }

    fn store_datalogger_settings(
        &mut self,
        bytes: &[u8; rriv_board::EEPROM_DATALOGGER_SETTINGS_SIZE],
    ) {
        eeprom::write_datalogger_settings_to_eeprom(self, bytes);
    }

    fn retrieve_datalogger_settings(
        &mut self,
        buffer: &mut [u8; rriv_board::EEPROM_DATALOGGER_SETTINGS_SIZE],
    ) {
        eeprom::read_datalogger_settings_from_eeprom(self, buffer);
    }

    fn retrieve_sensor_settings(
        // retrieve_all_sensor_configurations
        &mut self,
        buffer: &mut [u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE
                 * rriv_board::EEPROM_TOTAL_SENSOR_SLOTS],
    ) {
        for slot in 0..rriv_board::EEPROM_TOTAL_SENSOR_SLOTS {
            let slice = &mut buffer[slot * rriv_board::EEPROM_SENSOR_SETTINGS_SIZE
                ..(slot + 1) * rriv_board::EEPROM_SENSOR_SETTINGS_SIZE];
            read_sensor_configuration_from_eeprom(self, slot.try_into().unwrap(), slice);
        }
    }

    fn store_sensor_settings(
        &mut self,
        slot: u8,
        bytes: &[u8; rriv_board::EEPROM_SENSOR_SETTINGS_SIZE],
    ) {
        // sensor_configuration
        write_sensor_configuration_to_eeprom(self, slot, bytes);
    }

    fn delay_ms(&mut self, ms: u16) {
        self.delay.delay_ms(ms);
    }

    fn set_epoch(&mut self, epoch: i64) {
        let i2c1 = mem::replace(&mut self.i2c1, None);
        let mut ds3231 = Ds323x::new_ds3231(i2c1.unwrap());
        let millis = epoch * 1000;
        // DateTime::from_timestamp_millis(micros);
        let datetime = NaiveDateTime::from_timestamp_millis(millis);
        // defmt::println!("{:?}", datetime);
        if let Some(datetime) = datetime {
            match ds3231.set_datetime(&datetime) {
                Ok(_) => {}
                Err(err) => defmt::println!("Error set epoch {:?}", defmt::Debug2Format(&err)),
            }
        }
        let _result = ds3231.datetime();
        self.i2c1 = Some(ds3231.destroy_ds3231());
    }

    fn epoch_timestamp(&mut self) -> i64 {
        let i2c1 = mem::replace(&mut self.i2c1, None);
        let mut ds3231 = Ds323x::new_ds3231(i2c1.unwrap());
        let result = ds3231.datetime();
        self.i2c1 = Some(ds3231.destroy_ds3231());

        match result {
            Ok(date_time) => {
                // defmt::println!("got DS3231 time {:?}", date_time.and_utc().timestamp());
                date_time.and_utc().timestamp()
            }
            Err(err) => {
                defmt::println!("DS3231 error {:?}", defmt::Debug2Format(&err));
                return 0; // this could fail back to some other clock
            }
        }
    }

    // also crystal time, systick?

    fn timestamp(&mut self) -> i64 {
        return self.internal_rtc.current_time().into(); // internal RTC
    }

    fn get_millis(&mut self) -> u32 {
        let millis = self.counter.now();
        let millis = millis.ticks();
        millis
    }

    fn millis(&mut self) -> u32 {
        return self.get_millis();
    }

    fn get_battery_level(&mut self) -> i16 {
        match self
            .battery_level
            .measure_battery_level(&mut self.internal_adc, &mut self.delay)
        {
            Ok(value) => return value as i16,
            Err(_err) => return -1,
        }
    }

    fn sleep(&mut self) {
        // need to extend IndependentWatchdog to sleep the watch dog
        // self.watchdog.acc
        self.watchdog.feed();
    }

    fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    fn write_log_file(&mut self, args: fmt::Arguments) {
        // self.storage.write(data.as_bytes(), self.file_epoch);

        let mut buf = [0u8; 100];
        match format_no_std::show(
            &mut buf,
            args
        ) {
            Ok(string) => {
                if let Some(ref mut storage) = &mut self.storage {
                    storage.write(string.as_bytes(), self.file_epoch);
                }
            }
            Err(_) => {
                defmt::println!("format error writing log file")
            },
        }
    }

    fn flush_log_file(&mut self) {
        if let Some(ref mut storage) = &mut self.storage {
            storage.flush();
        }
    }

    fn dump_eeprom(&mut self) {
        let mut buffer: [u8; EEPROM_TOTAL_SENSOR_SLOTS * rriv_board::EEPROM_SENSOR_SETTINGS_SIZE] =
            [0; EEPROM_TOTAL_SENSOR_SLOTS * rriv_board::EEPROM_SENSOR_SETTINGS_SIZE];
        self.retrieve_sensor_settings(&mut buffer);

        for i in 0..buffer.len() {
            if i % rriv_board::EEPROM_SENSOR_SETTINGS_SIZE == 0 {
                rriv_board::RRIVBoard::usb_serial_send(
                    self,
                    format_args!("\n{}:", i / rriv_board::EEPROM_SENSOR_SETTINGS_SIZE),
                );
            }
            rriv_board::RRIVBoard::usb_serial_send(self, format_args!("{}", &buffer[i]));
        }
        rriv_board::RRIVBoard::usb_serial_send(self, format_args!("}}\n")); // } ends the transmissions
    }

    fn get_uid(&mut self) -> [u8; 12] {
        return self.uid;
    }

    fn set_serial_number(
        &mut self,
        serial_number: [u8; rriv_board::EEPROM_SERIAL_NUMBER_SIZE],
    ) -> bool {
        let existing_serial_number = self.get_serial_number();
        if existing_serial_number != [255, 255, 255, 255, 255] {
            return false;
        }
        eeprom::write_serial_number_to_eeprom(self, &serial_number);
        return true;
    }

    fn get_serial_number(&mut self) -> [u8; rriv_board::EEPROM_SERIAL_NUMBER_SIZE] {
        eeprom::read_serial_number_from_eeprom(self)
    }
    
    fn rs485_send(&mut self, message: &[u8]) {
        cortex_m::interrupt::free(|_cs| {
            for char in message.iter() {
            // rprintln!("char {}", char);
            _ = nb::block!( components::uart5::write(char.clone()));   
            }
        });
    }

    fn query_internal_adc(&mut self, channel: u8) -> u16 {
        match self.internal_adc.read(channel) {
            Ok(value) => return value,
            Err(error) => {
                let error_string = match error {
                    AdcError::NBError(_) => "Internal ADC NBError",
                    AdcError::NotConfigured => "Internal ADC Not Configured",
                    AdcError::ReadError => "Internal ADC Read Error",
                };
                rriv_board::RRIVBoard::usb_serial_send(self, format_args!("{}", &error_string));
                return 0;
            }
        }
    }

    fn query_external_adc(&mut self, channel: u8) -> u16 {
        let i2c1 = mem::replace(&mut self.i2c1, None);
        let mut i2c1 = i2c1.unwrap();
        let value = self.external_adc.read_single_channel(&mut i2c1, channel);
        self.i2c1 = Some(i2c1);
        return value;
    }

    fn read_temp_adc(&mut self) -> i32 {
        return self.internal_adc.read_tempertature();
    }

    fn ic2_read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), ()> {
        match self.i2c2.read(addr, buffer) {
            Ok(_) => return Ok(()),
            Err(e) => {
                rriv_board::RRIVBoard::serial_debug(
                    self,
                    format_args!("Problem reading I2C2 {:X?} {:?}", addr, e),
                );
                for i in 0..buffer.len() {
                    buffer[i] = 0b11111111; // error value
                }
                return Err(());
            }
        }
    }

    fn ic2_write(&mut self, addr: u8, message: &[u8]) -> Result<(), ()> {
        match self.i2c2.write(addr, message) {
            Ok(_) => return Ok(()),
            Err(e) => {
                let kind = match e {
                    stm32f1xx_hal::i2c::Error::Bus => "bus",
                    stm32f1xx_hal::i2c::Error::Arbitration => "arb",
                    stm32f1xx_hal::i2c::Error::Acknowledge => "ack",
                    stm32f1xx_hal::i2c::Error::Overrun => "ovr",
                    stm32f1xx_hal::i2c::Error::Timeout => "tout",
                    _ => "none",
                };
                rriv_board::RRIVBoard::serial_debug(
                    self,
                    format_args!("Problem writing I2C2 {:X?} {}", addr, kind),
                );
                return Err(());
            }
        }
    }

    fn ic2_write_read(&mut self, addr: u8, message: &[u8], buffer: &mut [u8]) -> Result<(), ()> {
        match self.i2c2.write_read(addr, message, buffer) {
            Ok(_) => return Ok(()),
            Err(e) => {
                let kind = match e {
                    stm32f1xx_hal::i2c::Error::Bus => "bus",
                    stm32f1xx_hal::i2c::Error::Arbitration => "arb",
                    stm32f1xx_hal::i2c::Error::Acknowledge => "ack",
                    stm32f1xx_hal::i2c::Error::Overrun => "ovr",
                    stm32f1xx_hal::i2c::Error::Timeout => "tout",
                    _ => "none",
                };
                rriv_board::RRIVBoard::serial_debug(
                    self,
                    format_args!("Problem writing I2C2 {:X?} {}", addr, kind),
                );
                return Err(());
            }
        }
    }

    fn one_wire_send_command(&mut self, command: u8, address: u64) {
        let address = Address(address);

        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.send_command(command, Some(&address), &mut self.precise_delay) {
                Ok(_) => defmt::println!("sent command ok"),
                Err(e) => defmt::println!("{:?}", defmt::Debug2Format(&e)),
            }
        } else {
            defmt::println!("one wire bus not available")
        }
    }

    fn one_wire_reset(&mut self) {
        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.reset(&mut self.precise_delay) {
                Ok(found_device) => {
                    if !found_device {
                        defmt::println!("no one wire device found");
                    }
                }
                Err(err) => defmt::println!("one_wire_reset: {:?}", defmt::Debug2Format(&err)),
            }
        } else {
            defmt::println!("one wire bus not available");
        }
    }

    fn one_wire_skip_address(&mut self) {
        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.skip_address(&mut self.precise_delay) {
                Ok(_) => {}
                Err(err) => defmt::println!("one_wire_skip_address: {:?}", defmt::Debug2Format(&err)),
            }
        } else {
            defmt::println!("one wire bus not available");
        }
    }

    fn one_wire_write_byte(&mut self, byte: u8) {
        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.write_byte(byte, &mut self.precise_delay) {
                Ok(_) => {}
                Err(err) => defmt::println!("one_wire_write_byte: {:?}", defmt::Debug2Format(&err)),
            }
        } else {
            defmt::println!("one wire bus not available");
        }
    }

    fn one_wire_match_address(&mut self, address: u64) {
        let address = Address(address);
        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.match_address(&address, &mut self.precise_delay) {
                Ok(_) => {}
                Err(err) => defmt::println!("one_wire_match_address: {:?}", defmt::Debug2Format(&err)),
            }
        } else {
            defmt::println!("one wire bus not available");
        }
    }

    fn one_wire_read_bytes(&mut self, output: &mut [u8]) -> Result<(), ()> {
        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.read_bytes(output, &mut self.precise_delay) {
                Ok(_) => {
                    defmt::println!("one_wire_read_bytes {:?}", output);
                }
                Err(err) => {
                    defmt::println!("one_wire_read_bytes {:?}", defmt::Debug2Format(&err));
                }
            }
        } else {
            defmt::println!("one wire bus not available");
        }
        // TODO
        if crc8(output) != 0 {
            defmt::println!("one wire bad CRC"); // how do we tell the caller??
            return Err(());
        }

        Ok(())
    }

    fn one_wire_bus_start_search(&mut self) {
        self.one_wire_search_state = None;
    }

    fn one_wire_bus_search(&mut self) -> Option<u64> {
        if let Some(one_wire_bus) = &mut self.one_wire_bus {
            match one_wire_bus.device_search(
                self.one_wire_search_state.as_ref(),
                false,
                &mut self.precise_delay,
            ) {
                Ok(Some((device_address, state))) => {
                    self.one_wire_search_state = Some(state);
                    return Some(device_address.0);
                }
                Ok(None) => {
                    defmt::println!("no more devices 1wire");
                    return None;
                }
                Err(e) => {
                    defmt::println!("1wire error{:?}", defmt::Debug2Format(&e));
                    return None;
                }
            }
        } else {
            defmt::println!("one wire bus not available");
            return None;
        }
    }

    fn write_gpio_pin(&mut self, pin: u8, value: bool) {
        match pin {
            1 => {
                let gpio = &mut self.gpio.gpio1;
                write_gpio!(gpio, value);
            }
            2 => {
                let gpio = &mut self.gpio.gpio2;
                write_gpio!(gpio, value);
            }
            // 3 => {
            //     let gpio = &mut self.gpio.gpio3;
            //     write_gpio!(gpio, value);
            // }
            // 4 => {
            //     let gpio = &mut self.gpio.gpio4;
            //     write_gpio!(gpio, value);
            // }
            5 => {
                let gpio = &mut self.gpio.gpio5;
                write_gpio!(gpio, value);
            }
            6 => {
                let gpio = &mut self.gpio.gpio6;
                write_gpio!(gpio, value);
            }
            7 => {
                let gpio = &mut self.gpio.gpio7;
                write_gpio!(gpio, value);
            }
            8 => {
                let gpio = &mut self.gpio.gpio8;
                write_gpio!(gpio, value);
            }
            _ => {
                let gpio = &mut self.gpio.gpio6;
                write_gpio!(gpio, value);
            }
        };
    }
    
    fn write_pwm_pin_duty(&mut self, value: u8){
    }


    fn read_gpio_pin(&mut self, pin: u8) -> Result<bool, ()> {
        match pin {
            1 => {
                let pin =  &mut self.gpio.gpio1;
                return read_pin!(pin);
            },
            2 => {
                let pin =  &mut self.gpio.gpio2;
                return read_pin!(pin);
            },
            3 => {
                let pin =  &mut self.gpio.gpio3;
                return read_pin!(pin);
            },
            4 => {
                let pin =  &mut self.gpio.gpio4;
                return read_pin!(pin);
            },
            5 => {
                let pin =  &mut self.gpio.gpio5;
                return read_pin!(pin);
            },
            6 => {
                let pin =  &mut self.gpio.gpio6;
                return read_pin!(pin);
            },
            7 => {
                let pin =  &mut self.gpio.gpio7;
                return read_pin!(pin);
            },
            8 => {
                let pin =  &mut self.gpio.gpio8;
                return read_pin!(pin);
            },
            _ => {
                return Err(());
            }
        }
    }
    
    fn set_gpio_pin_mode(&mut self, pin: u8, mode: rriv_board::gpio::GpioMode) {

        match pin {
            1 => {
                let cr = &mut self.gpio_cr.gpiob_crh;
                let pin: &mut Pin<'B', 8, Dynamic> = &mut self.gpio.gpio1;
                set_pin_mode!(pin, cr, mode);
            }
            2 => {
                let cr = &mut self.gpio_cr.gpiob_crl;
                let pin = &mut self.gpio.gpio2;
                set_pin_mode!(pin, cr, mode);
            }
            // 3 => {
            //     let cr = &mut self.gpio_cr.gpiob_crl;
            //     let pin = &mut self.gpio.gpio3;
            //     set_pin_mode!(pin, cr, mode);
            // }
            // 4 => {
            //     let cr = &mut self.gpio_cr.gpiob_crl;
            //     let pin = &mut self.gpio.gpio4;
            //     set_pin_mode!(pin, cr, mode);
            // }
            5 => {
                let cr = &mut self.gpio_cr.gpiod_crl;
                let pin = &mut self.gpio.gpio5;
                set_pin_mode!(pin, cr, mode);
            }
            6 => {
                let cr = &mut self.gpio_cr.gpioc_crh;
                let pin = &mut self.gpio.gpio6;
                set_pin_mode!(pin, cr, mode);
            }
            7 => {
                let cr = &mut self.gpio_cr.gpioc_crh;
                let pin = &mut self.gpio.gpio7;
                set_pin_mode!(pin, cr, mode);
            }
            8 => {
                let cr = &mut self.gpio_cr.gpioc_crh;
                let pin = &mut self.gpio.gpio8;
                set_pin_mode!(pin, cr, mode);
            }
            _ => {}
        }
    }
  
    fn disable_interrupts(&self) {
        self.disable_interrupts();
    }

    fn enable_interrupts(&self) {
        self.enable_interrupts();
    }

    fn get_errors(&self) -> [HardwareError; 5] {
        return self.hardware_errors;
    }

    fn error_alarm(&mut self) {
        // unsafely use the sd card pin to notify the user
        let slow = 25_u16;
        let fast = 10_u16;
        unsafe {
            let device_peripherals: pac::Peripherals = pac::Peripherals::steal();
            let mut gpioc = device_peripherals.GPIOC.split();
            let cs = gpioc.pc8;
            let mut cs = cs.into_push_pull_output(&mut gpioc.crh);
            for _i in 0..10 {
                cs.set_low();
                self.delay.delay_ms(fast);
                cs.set_high();
                self.delay.delay_ms(fast);
            }
            for _i in 0..5 {
                cs.set_low();
                self.delay.delay_ms(slow);
                cs.set_high();
                self.delay.delay_ms(slow);
            }
            for _i in 0..10 {
                cs.set_low();
                self.delay.delay_ms(fast);
                cs.set_high();
                self.delay.delay_ms(fast);
            }
        }
    }

}




#[interrupt]
unsafe fn USART2() {
    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut rx) = USART_RX.borrow(cs).borrow_mut().deref_mut() {
            if rx.is_rx_not_empty() {
                if let Ok(c) = nb::block!(rx.read()) {
                    defmt::println!("serial rx byte: {}", c);

                    let r = USART2_RX_PROCESSOR.borrow(cs);

                    if let Some(processor) = r.borrow_mut().deref_mut() {
                        processor.process_byte(c.clone());
                    }
                }
            }
        }
    })
}

#[interrupt]
fn USB_HP_CAN_TX() {
    cortex_m::interrupt::free(|cs| {
        dsb();
        usb_interrupt(cs);
        dmb();
    });
}

#[interrupt]
fn USB_LP_CAN_RX0() {
    cortex_m::interrupt::free(|cs| {
        dsb();
        usb_interrupt(cs);
        dmb();
    });
}

fn usb_interrupt(cs: &CriticalSection) {
    let usb_dev = unsafe { USB_DEVICE.as_mut().unwrap() };
    let serial = unsafe { USB_SERIAL.as_mut().unwrap() };

    if !usb_dev.poll(&mut [serial]) {
        return;
    }

    let mut buf = [0u8; 8];

    match serial.read(&mut buf) {
        Ok(count) if count > 0 => {
            // defmt::println!("count: {}", count);
            for c in buf[0..count].iter() {
                // defmt::println!("tx byte: {:X}", c);
                let r = RX_PROCESSOR.borrow(cs);
                if let Some(processor) = r.borrow_mut().deref_mut() {
                    processor.process_byte(c.clone());
                }
            }
            serial.write(&buf[0..count]).ok();
        }
        _ => {}
    }
}

pub fn build() -> Board {
    let mut board_builder = BoardBuilder::new();
    board_builder.setup();
    let board = board_builder.build();
    board
}

pub struct BoardBuilder {
    pub uid: Option<[u8; 12]>,

    // chip features
    pub delay: Option<DelayUs<TIM3>>,
    pub precise_delay: Option<PreciseDelayUs>,

    // pins groups
    pub gpio: Option<DynamicGpioPins>,
    pub gpio_cr: Option<GpioCr>,

    // board features
    pub internal_adc: Option<InternalAdc>,
    pub external_adc: Option<ExternalAdc>,
    pub power_control: Option<PowerControl>,
    pub oscillator_control: Option<OscillatorControl>,
    pub battery_level: Option<BatteryLevel>,
    pub rgb_led: Option<RgbLed>,
    pub i2c1: Option<BoardI2c1>,
    pub i2c2: Option<BoardI2c2>,
    pub internal_rtc: Option<Rtc>,
    pub storage: Option<Storage>,
    pub watchdog: Option<IndependentWatchdog>,
    pub counter: Option<CounterUs<TIM5>>,
    hardware_errors: [HardwareError; 5]
    pub clocks: Option<Clocks>
}

impl BoardBuilder {
    pub fn new() -> Self {
        BoardBuilder {
            uid: None,
            i2c1: None,
            i2c2: None,
            delay: None,
            precise_delay: None,
            gpio: None,
            gpio_cr: None,
            internal_adc: None,
            external_adc: None,
            power_control: None,
            battery_level: None,
            rgb_led: None,
            oscillator_control: None,
            internal_rtc: None,
            storage: None,
            watchdog: None,
            counter: None,
            hardware_errors: [HardwareError::None; 5]
            clocks: None
        }
    }

    pub fn build(self) -> Board {
        let mut gpio_cr = self.gpio_cr.unwrap();

        let mut one_wire = None;
        let mut internal_adc = self.internal_adc.unwrap();
        let pin = internal_adc.take_port_5();
        let mut pin = pin.into_dynamic(&mut gpio_cr.gpioc_crl);
        let _ = pin.set_high();
        pin.make_open_drain_output(&mut gpio_cr.gpioc_crl);
        let pin: OneWirePin<Pin<'C', 0, Dynamic>> = OneWirePin { pin };

        one_wire = match OneWire::new(pin) {
            Ok(one_wire) => Some(one_wire),
            Err(e) => {
                defmt::println!("{:?} bad one wire bus", defmt::Debug2Format(&e));
                panic!("bad one wire bus");
            }
        };

        let mut watchdog = self.watchdog.unwrap();
        watchdog.feed();

        Board {
            uid: self.uid.unwrap(),
            i2c1: self.i2c1,
            i2c2: self.i2c2.unwrap(),
            delay: self.delay.unwrap(),
            precise_delay: self.precise_delay.unwrap(),
            gpio: self.gpio.unwrap(),
            gpio_cr: gpio_cr,
            // // power_control: self.power_control.unwrap(),
            internal_adc: internal_adc,
            external_adc: self.external_adc.unwrap(),
            battery_level: self.battery_level.unwrap(),
            rgb_led: self.rgb_led.unwrap(),
            oscillator_control: self.oscillator_control.unwrap(),
            internal_rtc: self.internal_rtc.unwrap(),
            storage: self.storage,
            debug: false,
            file_epoch: 0,
            one_wire_bus: one_wire,
            one_wire_search_state: None,
            watchdog: watchdog,
            counter: self.counter.unwrap(),
            hardware_errors: self.hardware_errors
            clocks: self.clocks.unwrap(),
            pwm: None,
        }
    }

    fn setup_clocks(
        oscillator_control: &mut OscillatorControlPins,
        cfgr: CFGR,
        flash_acr: &mut ACR,
    ) -> Clocks {
        oscillator_control.enable_hse.set_high();

        // Freeze the configuration of all the clocks in the system
        // and store the frozen frequencies in `clocks`
        let clocks = cfgr
            .use_hse(HSE_MHZ.MHz())
            .sysclk(SYSCLK_MHZ.MHz())
            .pclk1(PCLK_MHZ.MHz())
            // .adcclk(14.MHz())
            .freeze(flash_acr);

        assert!(clocks.usbclk_valid());

        defmt::println!("{:?}", defmt::Debug2Format(&clocks));

        clocks
    }

    fn setup_serial(
        pins: pin_groups::SerialPins,
        mapr: &mut MAPR,
        usart: USART2,
        clocks: &Clocks,
    ) {
        // defmt::println!("initializing serial");

        let mut serial = Hal_Serial::new(
            usart,
            (pins.tx, pins.rx),
            mapr,
            // Config::default().baudrate(38400.bps()).wordlength_8bits().parity_none().stopbits(StopBits::STOP1), // this worked for the nox sensor
            Config::default()
                .baudrate(115200.bps())// this appears to be right for the RAK 3172
                // .baudrate(38400.bps()) // going slower for uart5 and rs485 for now
                .wordlength_8bits()
                .parity_none()
                .stopbits(StopBits::STOP1), 
            &clocks,
        );

        // defmt::println!("serial rx.listen()");

        serial.rx.listen();

        cortex_m::interrupt::free(|cs| {
            USART_RX.borrow(cs).replace(Some(serial.rx));
            USART_TX.borrow(cs).replace(Some(serial.tx));
            // WAKE_LED.borrow(cs).replace(Some(led)); // TODO: this needs to be updated.  entire rgb_led object needs to be shared.
        });
        // defmt::println!("unmasking USART2 interrupt");
        unsafe {
            NVIC::unmask(pac::Interrupt::USART2);
        }
    }

    fn setup_usb(pins: pin_groups::UsbPins, cr: &mut GpioCr, usb: USB, clocks: &Clocks) {
        // USB Serial
        let mut usb_dp = pins.usb_dp; // take ownership
        usb_dp.make_push_pull_output(&mut cr.gpioa_crh);
        let _ = usb_dp.set_low();
        delay(clocks.sysclk().raw() / 100);

        let usb_dm = pins.usb_dm;
        let usb_dp = usb_dp.into_floating_input(&mut cr.gpioa_crh);

        let usb = Peripheral {
            usb: usb,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        // Unsafe to allow access to static variables
        unsafe {
            let bus = UsbBus::new(usb);

            USB_BUS = Some(bus);

            USB_SERIAL = Some(SerialPort::new(USB_BUS.as_ref().unwrap()));

            let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x0483, 0x29))
                .manufacturer("RRIV")
                .product("RRIV Data Logger")
                .serial_number("_rriv")
                .device_class(USB_CLASS_CDC)
                .build();

            USB_DEVICE = Some(usb_dev);
        }

        unsafe {
            NVIC::unmask(pac::Interrupt::USB_HP_CAN_TX);
            NVIC::unmask(pac::Interrupt::USB_LP_CAN_RX0);
        }
    }

    pub fn setup_i2c1(
        pins: pin_groups::I2c1Pins,
        i2c1: I2C1,
        mapr: &mut MAPR,
        clocks: &Clocks,
    ) -> BoardI2c1 {
        let scl1 = pins.i2c1_scl;
        let sda1 = pins.i2c1_sda;

        BlockingI2c::i2c1(
            i2c1,
            (scl1, sda1),
            mapr,
            Mode::Standard {
                frequency: 100.kHz(), // slower to same some energy?
            },
            *clocks,
            1000,
            10,
            1000,
            1000,
        )
    }

    pub fn setup_i2c2(
        pins: pin_groups::I2c2Pins,
        cr: &mut GpioCr,
        i2c2: I2C2,
        clocks: &Clocks,
    ) -> BoardI2c2 {
        let scl2 = pins.i2c2_scl.into_alternate_open_drain(&mut cr.gpiob_crh); // i2c
        let sda2 = pins.i2c2_sda.into_alternate_open_drain(&mut cr.gpiob_crh); // i2c
        let x = BlockingI2c::i2c2(
            i2c2,
            (scl2, sda2),
            Mode::Standard {
                frequency: 100.kHz(), // slower to same some energy?
            },
            *clocks,
            1000,
            10,
            1000000,
            1000000,
        );

        // this works, so moving out and putting back could conceivably work.
        // let mut ds3231 = Ds323x::new_ds3231( x);
        // let result = ds3231.datetime();
        // x = ds3231.destroy_ds3231();

        // let mut ds3231 = Ds323x::new_ds3231( x);
        // let result = ds3231.datetime();
        // x = ds3231.destroy_ds3231();

        return x;
    }

    fn setup(&mut self) {
        defmt::println!("board builder setup");

        let mut core_peripherals: pac::CorePeripherals = cortex_m::Peripherals::take().unwrap();
        let device_peripherals: pac::Peripherals = pac::Peripherals::take().unwrap();

        let uid = Uid::fetch();
        defmt::println!("uid: {:X}", uid.bytes());
        self.uid = Some(uid.bytes());

        // mcu device registers
        let rcc = device_peripherals.RCC.constrain();
        let mut flash = device_peripherals.FLASH.constrain();
        let mut afio = device_peripherals.AFIO.constrain(); // Prepare the alternate function I/O registers

        let mut pwr = device_peripherals.PWR;
        let mut backup_domain = rcc.bkp.constrain(device_peripherals.BKP, &mut pwr);

        // Prepare the GPIO
        let gpioa: gpio::gpioa::Parts = device_peripherals.GPIOA.split();
        let gpiob = device_peripherals.GPIOB.split();
        let gpioc = device_peripherals.GPIOC.split();
        let gpiod = device_peripherals.GPIOD.split();

        // Set up pins
        let (pins, mut gpio_cr) = Pins::build(gpioa, gpiob, gpioc, gpiod, &mut afio.mapr);
        let (
            external_adc_pins,
            internal_adc_pins,
            battery_level_pins,
            mut dynamic_gpio_pins,
            i2c1_pins,
            i2c2_pins,
            mut oscillator_control_pins,
            mut power_pins,
            rgb_led_pins,
            serial_pins,
            _spi1_pins,
            spi2_pins,
            usb_pins,
        ) = pin_groups::build(pins, &mut gpio_cr);

        let clocks =
            BoardBuilder::setup_clocks(&mut oscillator_control_pins, rcc.cfgr, &mut flash.acr);

        dynamic_gpio_pins
            .gpio6
            .make_push_pull_output(&mut gpio_cr.gpioc_crh);

        // let mut high = true;
        let precise_delay = PreciseDelayUs::new();

        let mut delay: DelayUs<TIM3> = device_peripherals.TIM3.delay(&clocks);

        let mut watchdog = IndependentWatchdog::new(device_peripherals.IWDG);
        watchdog.stop_on_debug(&device_peripherals.DBGMCU, true);

        watchdog.start(MilliSeconds::secs(6));
        watchdog.feed();

        BoardBuilder::setup_serial(
            serial_pins,
            &mut afio.mapr,
            device_peripherals.USART2,
            &clocks,
        );

        self.internal_rtc = Some(Rtc::new(device_peripherals.RTC, &mut backup_domain)); // TODO: make sure LSE on and running?


        BoardBuilder::setup_usb(usb_pins, &mut gpio_cr, device_peripherals.USB, &clocks);
        usb_serial_send("{\"status\":\"usb started up\"}\n", &mut delay);

        let delay2: DelayUs<TIM2> = device_peripherals.TIM2.delay(&clocks);
        watchdog.start(MilliSeconds::secs(20));
        let storage = storage::build(spi2_pins, device_peripherals.SPI2, clocks, delay2);
        watchdog.start(MilliSeconds::secs(6));
        let storage = match storage {
            Ok(storage) => Some(storage),
            Err(hardware_error) => {
                add_hardware_error(&mut self.hardware_errors, hardware_error);
                None
            },
        };
        if storage.is_none() {
            // sd card library has no way to release the spi and pins
            // so unsafely get the cs pin and flash it
            unsafe {
                let device_peripherals: pac::Peripherals = pac::Peripherals::steal();
                let mut gpioc = device_peripherals.GPIOC.split();
                let cs = gpioc.pc8;
                let mut cs = cs.into_push_pull_output(&mut gpioc.crh);
                for i in 1..10 {
                    cs.set_high();
                    delay.delay_ms(100_u32);
                    cs.set_low();
                    delay.delay_ms(100_u32);
                }
                cs.set_high();
            }
        }

        self.external_adc = Some(ExternalAdc::new(external_adc_pins));
        self.external_adc.as_mut().unwrap().disable(&mut delay);

        power_pins.enable_3v.set_high();
        delay.delay_ms(500_u32);
        power_pins.enable_3v.set_low();
        delay.delay_ms(500_u32);
        power_pins.enable_3v.set_high();
        delay.delay_ms(500_u32);

        // external adc and i2c stability require these steps
        power_pins.enable_5v.set_high();
        delay.delay_ms(250_u32);
        self.external_adc.as_mut().unwrap().enable(&mut delay);
        self.external_adc.as_mut().unwrap().reset(&mut delay);

        // unsafe { NVIC::unmask(pac::interrupt::WWDG) }; // is this the EWI ?
        // NVIC::mask(pac::interrupt::WWDG);
        // device_peripherals.WWDG.cr.write( |w| unsafe { w
        //     .bits(0xFF) }
        // );
        // It is enabled by setting the WDGA bit in the
        // WWDG_CR register, then it cannot be disabled again except by a reset.
        // this might mean it's good to use the WWDG for general watchdog purposes, since it also has EWI
        // but actually... it also can't be disabled.
        //
        // the IndependentWatchdog won't trigger an interrupt.  it also can't be disabled.

        defmt::println!("unhang I2C1 if hung");

        let mut scl1 = i2c1_pins
            .i2c1_scl
            .into_open_drain_output(&mut gpio_cr.gpiob_crl);
        let mut sda1 = i2c1_pins
            .i2c1_sda
            .into_open_drain_output(&mut gpio_cr.gpiob_crl);
        sda1.set_high(); // remove signal from the master

        match try_unhang_i2c(
            &mut scl1,
            &sda1,
            &mut delay,
            i2c_hung_fix::FALLBACK_I2C_FREQUENCY,
            30,
        ) {
            Ok(_) => {}
            Err(_e) => {
                defmt::println!("Couln't reset i2c1");
                usb_serial_send("{\"status\":\"i2c1 failed, restarting\"}", &mut delay);
                loop {}
            } // wait for IDWP to reset.   actually we can just hardware reset here?
        }

        let i2c1_pins = I2c1Pins::rebuild(scl1, sda1, &mut gpio_cr);

        // defmt::println!("starting i2c");
        core_peripherals.DWT.enable_cycle_counter(); // BlockingI2c says this is required  already
        let mut i2c1 = BoardBuilder::setup_i2c1(
            i2c1_pins,
            device_peripherals.I2C1,
            &mut afio.mapr,
            &clocks,
        );
        defmt::println!("set up i2c1 done");

        // defmt::println!("skipping unhang I2C2 if hung");

        defmt::println!("unhang I2C2 if hung");

        let mut scl2 = i2c2_pins
            .i2c2_scl
            .into_open_drain_output(&mut gpio_cr.gpiob_crh);
        let mut sda2 = i2c2_pins
            .i2c2_sda
            .into_open_drain_output(&mut gpio_cr.gpiob_crh);
        sda2.set_high(); // remove signal from the master

        match try_unhang_i2c(
            &mut scl2,
            &sda2,
            &mut delay,
            100_000,
            i2c_hung_fix::RECOMMENDED_MAX_CLOCK_CYCLES,
        ) {
            Ok(ok) => {
                match ok {
                    i2c_hung_fix::Sucess::BusNotHung => {}
                    i2c_hung_fix::Sucess::FixedHungBus => {
                        defmt::println!("Fixed hung bus");
                        loop {} // wait for IDWD to reset
                    }
                }
            }
            Err(_) => {
                usb_serial_send("{\"status\":\"i2c2 failed, restarting\"}", &mut delay);
                loop {} // wait for IDWD to reset.   actually we can just hardware reset here?
            }
        }

        let i2c2_pins = I2c2Pins::rebuild(scl2, sda2, &mut gpio_cr);

        let mut i2c2 =
            BoardBuilder::setup_i2c2(i2c2_pins, &mut gpio_cr, device_peripherals.I2C2, &clocks);
        defmt::println!("set up i2c2 done");

        defmt::println!("i2c1 scanning...");

        for addr in 0x00_u8..0x7F {
            // Write the empty array and check the slave response.
            // defmt::println!("trying {:02x}", addr);
            let mut buf = [b'\0'; 1];
            if i2c1.read(addr, &mut buf).is_ok() {
                defmt::println!("{:02x} good", addr);
            }

            delay.delay_ms(10_u32);
        }
        defmt::println!("scan is done");

        watchdog.feed();

        defmt::println!("i2c2 scanning...");
        for addr in 0x00_u8..0x7F {
            // Write the empty array and check the slave response.
            // defmt::println!("trying {:02x}", addr);
            let mut buf = [b'\0'; 1];
            if i2c2.read(addr, &mut buf).is_ok() {
                defmt::println!("{:02x} good", addr);
            }
            delay.delay_ms(10_u32);
        }
        defmt::println!("scan is done");

        watchdog.feed();

        // configure external ADC
        self.external_adc.as_mut().unwrap().configure(&mut i2c1);

        self.i2c1 = Some(i2c1);
        self.i2c2 = Some(i2c2);

        // a basic idea is to have the struct for a given periphal take ownership of the register block that controls stuff there
        // then Board would have ownership of the feature object, and make changes to the the registers (say through shutdown) through the interface of that struct

        // build the power control
        let mut power_control = Some(PowerControl::new(power_pins)).unwrap();
        power_control.cycle_5v(&mut delay);

        // build the internal adc
        let internal_adc_configuration =
            InternalAdcConfiguration::new(internal_adc_pins, device_peripherals.ADC1);
        let mut internal_adc = internal_adc_configuration.build(&clocks);
        internal_adc.disable();
        delay.delay_ms(1000_u32);
        internal_adc.enable(&mut delay);
        self.internal_adc = Some(internal_adc);

        self.rgb_led = Some(build_rgb_led(
            rgb_led_pins,
            device_peripherals.TIM1,
            &mut afio.mapr,
            &clocks,
        ));

        self.battery_level = Some(BatteryLevel::new(battery_level_pins));

        self.oscillator_control = Some(OscillatorControl::new(oscillator_control_pins));

        self.gpio = Some(dynamic_gpio_pins);
        self.gpio_cr = Some(gpio_cr);

        self.storage = storage;

        self.delay = Some(delay);
        self.precise_delay = Some(precise_delay);

        // the millis counter
        let mut counter: CounterUs<TIM5> = device_peripherals.TIM5.counter_us(&clocks);
        match counter.start(2.micros()) {
            Ok(_) => defmt::println!("Millis counter start ok"),
            Err(err) => defmt::println!("Millis counter start not ok {:?}", defmt::Debug2Format(&err)),
        }
        self.counter = Some(counter);

        watchdog.feed();

        self.watchdog = Some(watchdog);


        self.clocks = Some(clocks);

        defmt::println!("done with setup");

    }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

pub fn usb_serial_send(string: &str, delay: &mut impl DelayMs<u16>) {
    cortex_m::interrupt::free(|_cs| {
        // USB
        let serial = unsafe { USB_SERIAL.as_mut().unwrap() };
        let bytes = string.as_bytes();
        let mut written = 0;

        let mut would_block_count = 0;
        while written < bytes.len() {
            match serial.write(&bytes[written..bytes.len()]) {
                Ok(bytes_written) => {
                    // defmt::println!("usb bytes written {}", bytes_written);
                    written = written + bytes_written;
                }
                Err(err) => {
                    match err {
                        UsbError::WouldBlock => {
                            if would_block_count > 100 {
                                defmt::println!("USBWouldBlock limit exceeded");
                                return;
                            }
                            would_block_count = would_block_count + 1; // handle hung blocking condition.  possibly caused by client not reading and buffer full.
                            delay.delay_ms(1);
                        }
                        _ => {
                            defmt::println!("usb error {:?}", defmt::Debug2Format(&err));
                        } // UsbError::ParseError => todo!(),
                          // UsbError::BufferOverflow => todo!(),
                          // UsbError::EndpointOverflow => todo!(),
                          // UsbError::EndpointMemoryOverflow => todo!(),
                          // UsbError::InvalidEndpoint => todo!(),
                          // UsbError::Unsupported => todo!(),
                          // UsbError::InvalidState => todo!(),
                    }
                }
            }
        }
    });
}

pub fn write_panic_to_storage(message: &str) {
    let device_peripherals = unsafe { pac::Peripherals::steal() };
    let rcc = device_peripherals.RCC.constrain();
    let mut flash = device_peripherals.FLASH.constrain();
    let mut afio = device_peripherals.AFIO.constrain(); // Prepare the alternate function I/O registers

    let clocks = rcc
        .cfgr
        .use_hse(HSE_MHZ.MHz())
        .sysclk(SYSCLK_MHZ.MHz())
        .pclk1(PCLK_MHZ.MHz())
        // .adcclk(14.MHz())
        .freeze(&mut flash.acr);

    let delay2: DelayUs<TIM2> = device_peripherals.TIM2.delay(&clocks);

    // Prepare the GPIO
    let gpioa: gpio::gpioa::Parts = device_peripherals.GPIOA.split();
    let gpiob = device_peripherals.GPIOB.split();
    let gpioc = device_peripherals.GPIOC.split();
    let gpiod = device_peripherals.GPIOD.split();

    // Set up pins
    let (pins, mut gpio_cr) = Pins::build(gpioa, gpiob, gpioc, gpiod, &mut afio.mapr);
    let (
        _external_adc_pins,
        _internal_adc_pins,
        _battery_level_pins,
        _dynamic_gpio_pins,
        _i2c1_pins,
        _i2c2_pins,
        mut _oscillator_control_pins,
        mut _power_pins,
        _rgb_led_pins,
        _serial_pins,
        _spi1_pins,
        spi2_pins,
        _usb_pins,
    ) = pin_groups::build(pins, &mut gpio_cr);

    let storage = storage::build(spi2_pins, device_peripherals.SPI2, clocks, delay2);
    match storage {
        Ok(mut storage) => {
            storage.create_file(0);
            storage.write(message.as_bytes(), 0);
            storage.flush();
        }
        Err(_) => {},
    }
}
