use alloc::format;

use ds323x::{Datelike, Timelike};
use embedded_hal::spi::{Mode, Phase, Polarity};
use embedded_sdmmc::{Directory, File, SdCard, TimeSource, Timestamp, Volume, VolumeManager};
use pac::SPI2;
use stm32f1xx_hal::spi::Spi2NoRemap;
// use embedded_sdmmc::{File, SdCard, TimeSource, Timestamp, Volume, VolumeManager};

use crate::*;

pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

pub fn build(
    pins: Spi2Pins,
    spi_dev: SPI2,
    clocks: Clocks,
    delay: Delay<TIM2, 1000000>,
) -> Storage {
    let spi2 = Spi::spi2(
        spi_dev,
        (pins.sck, pins.miso, pins.mosi),
        MODE,
        1.MHz(),
        clocks,
    );

    rprintln!("set up sdcard");
    // let sdmmc_spi = embedded_hal_bus::spi::RefCellDevice::new(&spi_bus, DummyCsPin, delay).unwrap();
    // only one SPI device on this bus, can we avoid using embedded_hal_bus?

    let sdcard = embedded_sdmmc::SdCard::new(spi2, pins.sd_card_chip_select, delay);

    return Storage::new(sdcard);
}

// type RrivSdCard = SdCard<Spi<SPI1, Spi1NoRemap, (Pin<'A', 5, Alternate>, Pin<'A', 6>, Pin<'A', 7, Alternate>), u8>, Pin<'C', 8, Output>, SysDelay>;
type RrivSdCard = SdCard<
    Spi<
        SPI2,
        Spi2NoRemap,
        (
            Pin<'B', 13, Alternate>,
            Pin<'B', 14>,
            Pin<'B', 15, Alternate>,
        ),
        u8,
    >,
    Pin<'C', 8, Output>,
    Delay<TIM2, 1000000>,
>;
// SdCard<Spi<SPI2, Spi2NoRemap, (Pin<'B', 13, Alternate>, Pin<'B', 14>, Pin<'B', 15, Alternate>), u8>, Pin<'C', 8, Output>>;
pub struct RrivTimeSource {}

impl RrivTimeSource {
    pub fn new() -> Self {
        RrivTimeSource {}
    }
}

// the Timesource trait below is a fraught way for us pass timestamps into the sd card library
// this cloud be refactored in the library to allow the caller to simply pass the timestamp
// into each write call, from whatever source it wants to use for a timestmap.
// for the time being, this static variable provides a way to transmit the timestamp into the library
// Additinally, in the case of the RRIVBoard, we do not want to query the I2C RTC every time we write
// therefore, this time should be coming form the internal RTC, which needs to be set to match that I2C RTC

static mut EPOCH_TIMESTAMP: i64 = 0;

impl TimeSource for RrivTimeSource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        let mut naive = NaiveDateTime::default();
        unsafe {
            naive = NaiveDateTime::from_timestamp(EPOCH_TIMESTAMP, 0);
        }
        let time = naive.time();
        let timestamp = embedded_sdmmc::Timestamp::from_calendar(
            naive.year().try_into().unwrap(),
            naive.month().try_into().unwrap(),
            naive.day().try_into().unwrap(),
            time.hour().try_into().unwrap(),
            time.minute().try_into().unwrap(),
            time.second().try_into().unwrap(),
        );
        match timestamp {
            Ok(timestamp) => timestamp,
            Err(_err) => Timestamp::from_calendar(0, 0, 0, 0, 0, 0).unwrap(),
        }
    }
}

pub trait OutputDevice {
    fn write(data: &[u8]);
}

const CACHE_SIZE: usize = 100;

pub struct Storage {
    volume_manager: VolumeManager<RrivSdCard, RrivTimeSource>,
    volume: Volume,
    filename: [u8; 11],
    file: Option<File>,
    root_dir: Directory,
    cache: [u8; CACHE_SIZE],
    next_position: usize,
}

impl Storage {
    pub fn new(
        sd_card: RrivSdCard, // , time_source: impl TimeSource //  a timesource passed in here could use unsafe access to internal RTC or i2c bus
    ) -> Self {
        let time_source = RrivTimeSource::new(); // unsafe access to the board
                                                 // or global time var via interrupt
                                                 // or copy into a global variable at the top of the run loop
        rprintln!("set up sdcard");

        let mut volume_manager = embedded_sdmmc::VolumeManager::new(sd_card, time_source);
        // Try and access Volume 0 (i.e. the first partition).
        // The volume object holds information about the filesystem on that volume.
        rprintln!("trying to set up sd card volume");
        let result = volume_manager.open_volume(embedded_sdmmc::VolumeIdx(0)); // TODO: this just hangs.  need window watchdog to catch here.
        let volume = match result {
            Ok(volume0) => {
                rprintln!("Volume Success: {:?}", volume0);
                volume0
            }
            Err(error) => {
                rprintln!("Volume error: {:?}", error);
                panic!("sd card error");
            }
        };

        // let volume = volume_manager.open_volume(embedded_sdmmc::VolumeIdx(0)).unwrap();
        rprintln!("Volume Success: {:?}", volume);
        // Open the root directory (mutably borrows from the volume).

        // let mut filename_bytes: [u8; 20] = [b'\0'; 20];
        // let filename = format!("{}.csv", timestamp).as_bytes();
        // filename_bytes[0..filename.len()].clone_from_slice(filename);
        // rprintln!("file: {:?}", filename);

        // rprintln!("set up root dir");

        let root_dir = volume_manager.open_root_dir(volume).unwrap();
        // This mutably borrows the directory.
        rprintln!("Root Dir: {:?}", root_dir);

        Storage {
            volume_manager,
            volume,
            filename: [b'\0'; 11],
            file: None,
            root_dir: root_dir,
            cache: [b'\0'; CACHE_SIZE],
            next_position: 0,
        }
    }

    pub fn reopen_file(&mut self) {
        let filename = match core::str::from_utf8(&self.filename) {
            Ok(filename) => filename,
            Err(err) => {
                rprintln!("{:?}", err);
                panic!();
            }
        };

        rprintln!("Filename: {:?}", filename);

        let my_file = match self.volume_manager.open_file_in_dir(
            self.root_dir,
            filename,
            embedded_sdmmc::Mode::ReadWriteCreateOrAppend,
        ) {
            Ok(my_file) => my_file,
            Err(error) => {
                rprintln!("{:?}", error);
                panic!();
            }
        };

        rprintln!("File: {:?}", my_file);

        self.file = Some(my_file);
    }

    pub fn create_file(&mut self, timestamp: i64) {
        let timestamp = if timestamp > 9999999 {
            timestamp & 9999999
        } else {
            timestamp
        };
        // let timestamp = timestamp > 1704067200 ? timestamp - 1704067200 : timestamp; // the RRIV epoch starts on Jan 1 2024, necessary to support short file names
        let filename = format!("{:0>7}.csv", timestamp);
        let filename = filename.as_bytes();
        rprintln!("file: {:?}", core::str::from_utf8(filename));
        let mut filename_bytes: [u8; 11] = [b'\0'; 11];
        filename_bytes[0..filename.len()].clone_from_slice(filename);
        self.filename = filename_bytes;

        // let my_file = self.volume_manager.open_file_in_dir(
        //   root_dir, output.as_str(), embedded_sdmmc::Mode::ReadWriteCreateOrAppend).unwrap();

        // self.reopen_file();
    }

    pub fn flush(&mut self) {
        if self.file == None {
            self.reopen_file();
        }

        if let Some(file) = self.file {
            let cache_data: &[u8] = &self.cache[0..self.next_position];
            match self.volume_manager.write(file, cache_data) {
                Ok(ret) => rprintln!("wrote: {:?}", ret),
                Err(err) => rprintln!("Err: {:?}", err),
            }

            match self.volume_manager.close_file(file) {
                Ok(_) => {}
                Err(err) => rprintln!("Err: {:?}", err),
            }
            self.reopen_file();
            self.next_position = 0;
            rprintln!("flushed");
        }
    }

    pub fn write(&mut self, data: &[u8], timestamp: i64) {
        //-> Result<Ok, Error<Error>>{

        // todo: what if data.len() is greater than the CACHE_SIZE

        // A hack to get the timestamp in to the SDCard library
        unsafe {
            EPOCH_TIMESTAMP = timestamp; // or function set_write_timestamp
        }

        let mut write_start = 0;

        if self.next_position + data.len() > CACHE_SIZE {
            // flush cache
            self.flush();
        }

        while data.len() - write_start > 0 {
            let mut write_length = data.len() - write_start;
            // rprintln!("writing {}", write_length);
            if write_length > CACHE_SIZE - self.next_position {
                write_length = CACHE_SIZE - self.next_position;
            }
            self.cache[self.next_position..self.next_position + write_length]
                .copy_from_slice(&data[write_start..(write_start + write_length)]);

            if self.next_position + data.len() > CACHE_SIZE {
                // flush cache
                self.flush();
            }

            self.next_position = self.next_position + write_length;
            write_start = write_start + write_length;
        }
    }
}
