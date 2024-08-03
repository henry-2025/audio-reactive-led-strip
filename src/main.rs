mod args;
mod audio;
mod config;
mod dsp;
mod gamma_table;
mod gui;
mod led;

use args::Args;
use clap::Parser;
use config::{load_config, DEFAULT_CONFIG_PATH};
use iced::Application;

pub fn main() -> iced::Result {
    let args = Args::parse();
    let mut config = load_config(&DEFAULT_CONFIG_PATH.to_string(), true);
    config.merge_with_args(args);
    print!("{:?}", config);

    if config.use_gui {
        gui::Gui::run(iced::Settings::with_flags(config.into()))
    } else {
        print!("default loop");
        Ok(())
    }
}
