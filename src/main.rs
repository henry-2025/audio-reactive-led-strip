mod args;
mod audio;
mod config;
mod dsp;
mod gamma_table;
mod led;
mod renderer;

use args::Args;
use clap::Parser;
use config::{load_config, DEFAULT_CONFIG_PATH};
use renderer::{Renderer, SharedRenderState};
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "cli"))]
mod gui;
#[cfg(not(feature = "cli"))]
use iced::Application;

#[cfg(feature = "cli")]
use std::sync::mpsc;

pub fn main() -> iced::Result {
    let args = Args::parse();
    let mut config = load_config(&DEFAULT_CONFIG_PATH.to_string(), true);
    config.merge_with_args(args);

    let render_state = Arc::new(Mutex::new(SharedRenderState::new(&config)));
    let renderer = Renderer::new(render_state.clone());

    #[cfg(not(feature = "cli"))]
    {
        return gui::Gui::run(iced::Settings::with_flags(config.into()));
    }

    #[cfg(feature = "cli")]
    {
        let (tx, rx) = mpsc::channel();

        ctrlc::set_handler(move || {
            println!("Ctrl+C received, signaling stop");
            tx.send(()).unwrap();
        })
        .expect("error setting up signal handler");
        renderer.main_loop(rx);
        return Ok(());
    }
}
