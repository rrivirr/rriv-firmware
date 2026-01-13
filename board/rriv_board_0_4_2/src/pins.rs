use stm32f1xx_hal::{gpio::*, afio::MAPR};





// to be consumed by other objects

// pub sd_card_chip_select: Pin<'C', 8>,




pub struct FreedDebugPins {
    pub pb3: Pin<'B', 3>,
    pub pb4: Pin<'B', 4>,
    #[allow(unused)]
    pub pa15: Pin<'A', 15>,
}

impl FreedDebugPins {
    pub fn build(
        pins: (
            Pin<'A', 15, Input<Floating>>,
            Pin<'B', 3, Input<Floating>>,
            Pin<'B', 4, Input<Floating>>,
        ),
    ) -> Self {
        FreedDebugPins {
            pb3: pins.1,
            pb4: pins.2,
            pa15: pins.0,
        }
    }
}


pub struct GpioCr {
    pub gpioa_crl: Cr<'A', false>,
    pub gpioa_crh: Cr<'A', true>,
    pub gpiob_crl: Cr<'B', false>,
    pub gpiob_crh: Cr<'B', true>,
    pub gpioc_crl: Cr<'C', false>,
    pub gpioc_crh: Cr<'C', true>,
    pub gpiod_crl: Cr<'D', false>,
}

pub struct Pins {
    pub internal_adc1: Pin<'A', 0>,
    pub enable_avdd: Pin<'A', 1>,
    pub tx: Pin<'A', 2>,
    pub rx: Pin<'A', 3>,
    pub external_adc_reset: Pin<'A', 4>,
    pub spi1_sck: Pin<'A', 5>,
    pub spi1_miso: Pin<'A', 6>,
    pub spi1_mosi: Pin<'A', 7>,
    pub rgb_red_and_wake_button: Pin<'A', 8>, // multi-use pin
    pub rgb_green_and_spi2_chip_select: Pin<'A', 9>, // multi-use pin
    pub rgb_blue: Pin<'A', 10>,
    pub usb_n: Pin<'A', 11>,
    pub usb_p: Pin<'A', 12>,
    // unused: Pin<'A', 13>, // swdio
    // unused: Pin<'A', 14>, // swclk
    // unused: Pin<'A', 15>,
    pub vin_measure: Pin<'B', 0>,
    pub enable_vin_measure: Pin<'B', 1>,
    //unused: Pin<'B', 2>, // boot1
    pub gpio4: Pin<'B', 3>,
    pub gpio3: Pin<'B', 4>,
    pub gpio2: Pin<'B', 5>,
    pub i2c1_scl: Pin<'B', 6>,
    pub i2c1_sda: Pin<'B', 7>,
    pub gpio1: Pin<'B', 8>,
    // unused: tm32f1xx_hal::gpio::Pin<'B', 9>,
    pub i2c2_scl: Pin<'B', 10>,
    pub i2c2_sda: Pin<'B', 11>,
    pub enable_5v: Pin<'B', 12>,
    pub spi2_sck: Pin<'B', 13>,
    pub spi2_miso: Pin<'B', 14>,
    pub spi2_mosi: Pin<'B', 15>,

    pub internal_adc5: Pin<'C', 0>,
    pub internal_adc4: Pin<'C', 1>,
    pub internal_adc3: Pin<'C', 2>,
    pub internal_adc2: Pin<'C', 3>,
    // unused: Pin<'C', 4>,
    pub enable_3v: Pin<'C', 5>,
    pub enable_external_adc: Pin<'C', 6>,
    // rgb_red_and_wake_duplicate: Pin<'C', 7>,  // trace accidentally duplicated on PCB
    pub sd_card_chip_select: Pin<'C', 8>,
    // unused: Pin<'C', 9>,
    pub gpio8: Pin<'C', 10>,
    pub gpio7: Pin<'C', 11>,
    pub gpio6: Pin<'C', 12>,
    pub enable_hse: Pin<'C', 13>, // high speed external oscillator on/off
    // unused: Pin<'C', 14>,
    // unused: Pin<'C', 15>,

    // unused: Pin<'D', 0>, // osc in
    // unused: Pin<'D', 1>, // osc out
    pub gpio5: Pin<'D', 2>,
}




impl Pins {
    pub fn build(
      gpioa: gpioa::Parts,
      gpiob: gpiob::Parts,
      gpioc: gpioc::Parts,
      gpiod: gpiod::Parts,  
      mapr: &mut MAPR
    ) -> (Self, GpioCr) {

        let gpio_cr = GpioCr {
            gpioa_crl: gpioa.crl,
            gpioa_crh: gpioa.crh,
            gpiob_crl: gpiob.crl,
            gpiob_crh: gpiob.crh,
            gpioc_crl: gpioc.crl,
            gpioc_crh: gpioc.crh,
            gpiod_crl: gpiod.crl,
        };

        let debug_pins =
        mapr
            .disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);
        let freed_debug_pins = FreedDebugPins::build(debug_pins);

        let pins = Pins {
            internal_adc1: gpioa.pa0,
            enable_avdd: gpioa.pa1,
            tx: gpioa.pa2,
            rx: gpioa.pa3,
            external_adc_reset: gpioa.pa4,
            spi1_sck: gpioa.pa5,
            spi1_miso: gpioa.pa6,
            spi1_mosi: gpioa.pa7,
            rgb_red_and_wake_button: gpioa.pa8,
            // wake_button: gpioa.pa8, // multi-use pin
            rgb_green_and_spi2_chip_select: gpioa.pa9,
            //spi2_chip_select: gpioa.pa9, // multi-use pin
            rgb_blue: gpioa.pa10,
            usb_n: gpioa.pa11,
            usb_p: gpioa.pa12,

            vin_measure: gpiob.pb0,
            enable_vin_measure: gpiob.pb1,
            gpio4: freed_debug_pins.pb3,
            gpio3: freed_debug_pins.pb4,
            gpio2: gpiob.pb5,
            i2c1_scl: gpiob.pb6,
            i2c1_sda: gpiob.pb7,
            gpio1: gpiob.pb8,
            i2c2_scl: gpiob.pb10,
            i2c2_sda: gpiob.pb11,
            enable_5v: gpiob.pb12,
            spi2_sck: gpiob.pb13,
            spi2_miso: gpiob.pb14,
            spi2_mosi: gpiob.pb15,

            internal_adc5: gpioc.pc0,
            internal_adc4: gpioc.pc1,
            internal_adc3: gpioc.pc2,
            internal_adc2: gpioc.pc3,
            enable_3v: gpioc.pc5,
            enable_external_adc: gpioc.pc6,
            sd_card_chip_select: gpioc.pc8,
            gpio8: gpioc.pc10,
            gpio7: gpioc.pc11,
            gpio6: gpioc.pc12,
            enable_hse: gpioc.pc13,
            gpio5: gpiod.pd2,
        };

        (pins, gpio_cr)
    }

}
