

use alloc::boxed::Box;


pub fn encode( timestamp: i64, 
               values: &[i16]
             ) -> Box<[u8]>{
    
    defmt::println!("encode {} {}", timestamp, values[0]);
    // defmt::println!("{} values", values.len());
    let mut bytes: [u8; 22] = [0; 22];
    // let timestamp_bytes = timestamp.to_le_bytes();
    // defmt::println!("{:X?}", timestamp_bytes);
    let timestamp_bytes = timestamp.to_be_bytes();
    defmt::println!("{:X}", timestamp_bytes);
    bytes[0..8].copy_from_slice(&timestamp_bytes);
    for i in 0..values.len(){
      let value = values[i];
      let value_bytes = value.to_be_bytes();
      defmt::println!("{:?}", (i * 4 + 8)..(i * 4 + 12));
      bytes[(i * 4 + 8)..(i * 4 + 12)].copy_from_slice(&value_bytes);
      defmt::println!("{:X}", value_bytes);
      if i == 2 { break }; // send up to 3 values
    }
    
    defmt::println!("{:X}", bytes);
    return Box::new(bytes);
}