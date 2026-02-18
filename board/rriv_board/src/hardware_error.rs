#[derive(Copy, Clone)]
pub enum HardwareError {
    None,
    StorageFull,
    StorageMissing,
    StorageOther,
}