
use core::i16::MAX;

use alloc::boxed::Box;

const MAX_VALUES: usize = 8*3*2 + 6; //
const BYTES_PER_VALUE: usize = 2;
pub const MAX_BYTES: usize = 8 + MAX_VALUES * BYTES_PER_VALUE;

pub fn encode( timestamp: i64, 
               values: &[i16],
               bytes: &mut [u8; MAX_BYTES]
             ) -> usize {
    
    defmt::println!("encode {} {}", timestamp, values[0]);
    let timestamp_bytes = timestamp.to_be_bytes();
    defmt::println!("{:X}", timestamp_bytes);
    bytes[0..8].copy_from_slice(&timestamp_bytes);
    let end = match values.iter().position(|&x| x == MAX) {
        Some(last) => last,
        None => MAX_VALUES,
    };
    
    for i in 0..end {
      let value_bytes = values[i].to_be_bytes();
      // rprintln!("{:?}", (i * 4 + 8)..(i * 4 + 12));
      let byte_position = 8 + i * BYTES_PER_VALUE;
      bytes[(byte_position)..(byte_position + BYTES_PER_VALUE)].copy_from_slice(&value_bytes);
      defmt::println!("{:X}", value_bytes);
    }
    
    defmt::println!("{:X}", bytes);
    return 8 + end * 2;
}