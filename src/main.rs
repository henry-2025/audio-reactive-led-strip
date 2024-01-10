use core::fmt;
use std::fmt::Formatter;

mod config;
mod dsp;
mod led;

const DEFAULT_CONFIG_PATH: &str = "$HOME/reactive.conf";

#[derive(Debug)]
struct Error {
    error_message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Exited with error: {:?}", self.error_message)
    }
}

fn main() {
    //let conf = config::load_config(&String::from(DEFAULT_CONFIG_PATH)).then;
}
