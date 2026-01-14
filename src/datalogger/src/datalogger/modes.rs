
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum DataLoggerMode {
    Interactive = 1,
    Field = 2,
    HibernateUntil = 3
}

impl DataLoggerMode {
  pub(crate) fn to_u8(&self) -> u8 {
        *self as u8
  }

  pub(crate) fn from_u8(index: u8) -> DataLoggerMode{
    match index {
        2 => DataLoggerMode::Field,
        3 => DataLoggerMode::HibernateUntil,
        _ => DataLoggerMode::Interactive
    }
  }
}

pub fn mode_text(mode: &DataLoggerMode) -> &'static str {
    match mode {
        DataLoggerMode::Interactive => "interactive",
        DataLoggerMode::Field => "field",
        DataLoggerMode::HibernateUntil => "hibernate",
    }
}


pub enum DataLoggerSerialTxMode { // 
    Normal,
    Quiet,
    Watch,
    #[allow(unused)]
    Interactive
}



        // DataLoggerSerialTxMode::Watch => "watch",
        // DataLoggerSerialTxMode::Quiet => "quiet",