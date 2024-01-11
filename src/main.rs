use core::fmt;
use std::fmt::Formatter;

mod audio;
mod config;
mod dsp;
mod gamma_table;
mod led;

const DEFAULT_CONFIG_PATH: &str = "$HOME/reactive.conf";

use gtk::builders::ApplicationBuilder;
use gtk::glib::subclass::object::ObjectImpl;
use gtk::glib::subclass::types::ObjectSubclass;
use gtk::subclass::range::RangeImpl;
use gtk::subclass::scale::ScaleImpl;
use gtk::subclass::widget::WidgetImpl;
use gtk::{glib, Application, Button};
use gtk::{prelude::*, ApplicationWindow};

const APP_ID: &str = "org.gtk_rs.HelloWorld1";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

// for starters, let's build a gui that has a slider and a gl context or cairo rendering area
// figure out the most conventional way to design the gui. Maybe declaratively maybe with a builder
// thing
// then build the two-node slider
// then create a dialog for mode selection--probably a dropdown
// then figure out how to render the spectrum to the graphical display
//

#[derive(Default)]
pub struct DualSlider;

#[glib::object_subclass]
impl ObjectSubclass for DualSlider {
    const NAME: &'static str = "GtkDualSlider";
    type Type = DualSlider;
    type ParentType = gtk::Scale;
}

impl ObjectImpl for DualSlider {}

impl WidgetImpl for DualSlider {}

impl ScaleImpl for DualSlider {
    fn layout_offsets(&self) -> (i32, i32) {
        self.parent_layout_offsets()
    }
}

impl RangeImpl for DualSlider {}

fn build_ui(app: &Application) {
    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    button.connect_clicked(|button| {
        button.set_label("Hello world!");
    });
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My Gtk App")
        .child(&button)
        .build();

    window.present();
}
