mod args;
mod audio;
mod config;
mod dsp;
mod gamma_table;
mod gui;
mod led;
mod renderer;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use args::Args;
use clap::Parser;
use config::{load_config, DEFAULT_CONFIG_PATH};
use iced::Application;
use renderer::{Renderer, SharedRenderState};

pub fn main() -> iced::Result {
    let args = Args::parse();
    let mut config = load_config(&DEFAULT_CONFIG_PATH.to_string(), true);
    config.merge_with_args(args);

    let render_state = Arc::new(Mutex::new(SharedRenderState::new(&config)));
    let renderer = Renderer::new(render_state.clone());

    if config.use_gui {
        gui::Gui::run(iced::Settings::with_flags(config.into()))
    } else {
        renderer.main_loop();
        thread::sleep(Duration::from_secs(10));
        Ok(())
    }
}
