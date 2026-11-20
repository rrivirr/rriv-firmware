const CHIP_ADDRESS = 0x7345

fn setup(&mut self, board: &mut dyn rriv_board::RRIVBoard) {

  
    let mut data = [0x00; 1];
    match board.ic2_read(CHIP_ADDRESS, &mut data){
        Ok(chip_id) => {
            //chip id read
        },
        Err(err) => {
            defmt::println!("Failed write to AHT20 {:?}", err);
            self.humidity = MAX;
            self.temperature = MAX
        },
    }

}