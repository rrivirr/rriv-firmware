// https://ferrous-systems.com/blog/test-embedded-app/

// use rtt_target::defmt::println;



pub const BUFFER_NUM: usize = 3; // Includes an extra empty cell for end marker
pub const BUFFER_SIZE: usize = 500;

pub struct CommandData {
    receiving: bool,
    buffer: [[u8; BUFFER_SIZE]; BUFFER_NUM],
    cur: usize,
    end: usize,
    command_pos: usize,
}

impl CommandData {
    pub const fn default() -> Self {
        Self {
            receiving: false,
            cur: 0,
            end: BUFFER_NUM - 1,
            command_pos: 0,
            buffer: [[b'\0'; BUFFER_SIZE]; BUFFER_NUM],
        }
    }
}

pub struct CommandRecognizer {}
impl CommandRecognizer {
    pub fn process_character(command_data: &mut CommandData, character: u8) {
        let receiving = command_data.receiving;
        let starting = character == b'{';

        if receiving && starting {
            // meaningless character
            return;
        }

        if receiving && (character == b'\r' || character == b'\n') {
            command_data.receiving = false;
            command_data.cur = (command_data.cur + 1) % BUFFER_NUM;
            return;
        }

        if starting {
            if command_data.cur == command_data.end {
                // circular buffer is full
                return;
            }
            command_data.receiving = true;
            command_data.command_pos = 0;
        }

        if !receiving && !starting {
            return;
        }

        let cur = command_data.cur;
        let pos: usize = command_data.command_pos;
        command_data.buffer[cur][pos] = character;
        if pos < BUFFER_SIZE - 1 {
            command_data.command_pos = command_data.command_pos + 1;
        }
    }

    pub fn pending_message_count(command_data: &CommandData) -> usize {
        if command_data.cur >= (command_data.end + 1) % BUFFER_NUM {
            return command_data.cur - (command_data.end + 1) % BUFFER_NUM;
        } else {
            return command_data.cur + BUFFER_NUM - (command_data.end + 1) % BUFFER_NUM;
        }
    }    

    pub fn take_command(command_data: &mut CommandData) -> [u8; BUFFER_SIZE] {
        // clone the command bytes buffer so the caller isn't borrowing the command_data buffer
        let buffer_index = (command_data.end + 1) % BUFFER_NUM;
        let command = command_data.buffer[buffer_index].clone();
        // null the buffer
        command_data.buffer[buffer_index] = [b'\0'; BUFFER_SIZE];
        // move the end marker, effectively marking the buffer as ready for use again
        command_data.end = buffer_index;

        return command;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_receiving() {
        let mut command_data: CommandData = CommandData::default();
        CommandRecognizer::process_character(&mut command_data, b'{');

        assert_eq!(true, command_data.receiving);
    }

    #[test]
    fn test_receiving_done() {
        let mut command_data: CommandData = CommandData::default();
        CommandRecognizer::process_character(&mut command_data, b'{');
        CommandRecognizer::process_character(&mut command_data, b'\r');

        assert_eq!(false, command_data.receiving);
    }

    #[test]
    fn test_message_ready() {
        let mut command_data = CommandData::default();
        CommandRecognizer::process_character(&mut command_data, b'{');
        CommandRecognizer::process_character(&mut command_data, b'\r');
        assert_eq!(true, command_data.message_ready)
    }

    #[test]
    fn test_message_saved() {
        let mut command_data = CommandData::default();
        let command = "{\"cmd\":\"set\",\"object\":\"sensor\"}\r";
        for c in command.bytes() {
            CommandRecognizer::process_character(&mut command_data, c);
        }

        assert_eq!(true, command_data.message_ready);

        println!("original val:  {}", command);
        println!(
            "processed val: {}",
            core::str::from_utf8(&command_data.buffer[0]).unwrap()
        );

        let mut matching = true;
        for (i, c) in command.bytes().enumerate() {
            if c == b'\r' {
                break;
            }
            if command_data.buffer[0][i] != c {
                println!("// {} {}", c, command_data.buffer[0][i]);
                matching = false;
            }
        }
        assert_eq!(true, matching);
    }

    #[test]
    fn test_multiple_messages() {
        let mut command_data = CommandData::default();
        let command = "{\"cmd\":\"set\",\"object\":\"sensor\"}\r";
        let command2 = "{\"cmd\":\"set\",\"object\":\"actuator\"}\r";
        for c in command.bytes() {
            CommandRecognizer::process_character(&mut command_data, c);
        }
        for c in command2.bytes() {
            CommandRecognizer::process_character(&mut command_data, c);
        }
        // check that there are commands ready
        assert_eq!(true, command_data.message_ready);
        // check that there are two commands ready
        assert_eq!(2, CommandRecognizer::pending_message_count(&command_data));
        // check that the first command is correct
        let mut matching = true;
        for (i, c) in command.bytes().enumerate() {
            if c == b'\r' {
                break;
            }
            if command_data.buffer[0][i] != c {
                println!("// {} {}", c, command_data.buffer[0][i]);
                matching = false;
            }
        }
        assert_eq!(true, matching);
    }

    #[test]
    fn test_many_messages() {
        let mut command_data = CommandData::default();
        let command = "{\"cmd\":\"set\",\"object\":\"sensor\"}\r";

        for _i in 0..10 {
            for c in command.bytes() {
                CommandRecognizer::process_character(&mut command_data, c);
            }
        }
        println!(
            "count {}",
            CommandRecognizer::pending_message_count(&command_data)
        );
        assert_eq!(10, CommandRecognizer::pending_message_count(&command_data))
    }
}
