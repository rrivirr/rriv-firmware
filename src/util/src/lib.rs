#![no_std]

use core::{str::Utf8Error};
use core::fmt::Debug;

pub fn remove_invalid_utf8(buffer: &mut [u8]) {
    // make sure all bytes are utf8 compliant
    for i in 0..buffer.len() {
        if buffer[i] > 0x7F {
            buffer[i] = 42; // change to star character.
        }
    }
}

pub fn str_from_utf8( buffer: &mut [u8] )-> Result<&str, Utf8Error> {
    remove_invalid_utf8(buffer);
    let nul_range_end = buffer.iter()
        .position(|&c| c == b'\0')
        .unwrap_or(buffer.len());
    core::str::from_utf8(&buffer[0..nul_range_end])
}



pub fn check_alphanumeric(array: &[u8]) -> bool {
    let checks = array.iter();
    let checks = checks.map(|x| (*x as char).is_alphanumeric() || *x == 0 || *x == b'_' || *x == b'-');
    let rval = checks.fold(true, |acc, check| if !check || !acc { false } else { true });
    rval && (array[0] != 0)
}

pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}


pub fn format_error<'a>(error: &'a dyn Debug, buffer: &'a mut [u8]) -> &'a str {
    match format_no_std::show(
        buffer,
        format_args!("{:?}", error)
    ) {
        Ok(message) => {
            return message;
        }
        Err(e) => {
            defmt::println!("{:?}", defmt::Debug2Format(&e));
            return "format error";
        },
    }
}