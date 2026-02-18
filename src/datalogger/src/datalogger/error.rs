use rriv_board::hardware_error::HardwareError;


pub fn hardware_error_text(error: HardwareError) -> &'static str{
    match error {
        HardwareError::None => "None",
        HardwareError::StorageFull => "SD Card Full",
        HardwareError::StorageMissing => "SD Card Missing",
        HardwareError::StorageOther => "SD Card Error",
    }
}