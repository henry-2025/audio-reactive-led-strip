mod args;
mod audio;
mod config;
mod dsp;
mod gamma_table;
#[cfg(feature = "gui")]
mod gui;
mod led;
mod renderer;

use std::sync::{mpsc, Arc, Mutex};

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

    #[cfg(feature = "gui")]
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
