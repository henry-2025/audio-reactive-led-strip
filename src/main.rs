mod audio;
mod config;
mod dsp;
mod gamma_table;
mod gui;
mod led;

use config::Config;
use iced::Application;

pub fn main() -> iced::Result {
    gui::Gui::run(iced::Settings::with_flags(Config::default().into()))
}

// for starters, let's build a gui that has a slider and a gl context or cairo rendering area
// figure out the most conventional way to design the gui. Maybe declaratively maybe with a builder
// thing
// then build the two-node slider
// then create a dialog for mode selection--probably a dropdown
// then figure out how to render the spectrum to the graphical display
//
