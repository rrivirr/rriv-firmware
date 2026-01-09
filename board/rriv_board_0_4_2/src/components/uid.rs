use core::ptr;


pub struct Uid {
    off0: u16,
    off2: u16,
    off4: u32,
    off8: u32,
}


impl Uid {
    pub fn fetch() -> Uid {
        let base = 0x1ffff7e8;

        let address0 = base as *const u16;
        let address2 = (base + 2) as *const u16;
        let address4 = (base + 4) as *const u32;
        let address8 = (base + 8) as *const u32;

        let value: u16 = unsafe { ptr::read(address0) };

        defmt::println!("Value at address: {}", value);

        Self {
            off0: unsafe { ptr::read(address0) },
            off2: unsafe { ptr::read(address2) },
            off4: unsafe { ptr::read(address4) },
            off8: unsafe { ptr::read(address8) }
        }
        
    }

    pub fn bytes(&self) -> [u8; 12] {
        let mut bytes: [u8; 12] = [0; 12];
        bytes[0..2].copy_from_slice(&mut self.off0.to_be_bytes());
        bytes[2..4].copy_from_slice(&mut self.off2.to_be_bytes());
        bytes[4..8].copy_from_slice(&mut self.off4.to_be_bytes());
        bytes[8..12].copy_from_slice(&mut self.off8.to_be_bytes());
        bytes
    }
}