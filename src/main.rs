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
use gui::Gui;

#[cfg(not(feature = "cli"))]
mod gui;
#[cfg(not(feature = "cli"))]
#[cfg(feature = "cli")]
use std::sync::mpsc;

pub fn main() -> iced::Result {
    #[cfg(not(feature = "cli"))]
    {
        iced::application("Audio Reactive Renderer", Gui::update, Gui::view)
            .subscription(Gui::subscription)
            .exit_on_close_request(false)
            .run()
    }

    #[cfg(feature = "cli")]
    {
        let render_state = Arc::new(Mutex::new(SharedRenderState::new(&config)));
        let renderer = Renderer::new(render_state.clone());
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
