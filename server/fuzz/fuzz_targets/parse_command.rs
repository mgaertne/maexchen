#![no_main]

use libfuzzer_sys::fuzz_target;
use maexchen::ClientCommand;

fuzz_target!(|text: &str| {
    if let Ok(command) = text.parse::<ClientCommand>() {
        assert_eq!(command.to_string().parse::<ClientCommand>(), Ok(command));
    }
});
