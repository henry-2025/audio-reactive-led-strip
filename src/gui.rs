mod double_slider;
pub mod waveform;

use clap::Parser;
use double_slider::DoubleSlider;
use double_slider::SliderSide;
use iced::futures::channel::mpsc;
use iced::futures::channel::mpsc::Receiver;
use iced::futures::channel::mpsc::Sender;
use iced::futures::SinkExt;
use iced::window;
use iced::Task;
use iced::{
    futures::Stream,
    widget::{column, horizontal_space, pick_list, row, shader},
};
use iced::{Alignment, Length, Subscription};
use ndarray::Array2;
use std::fmt::Display;
use std::sync;
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;
use waveform::pipeline::Vertex;
use waveform::Waveform;

use crate::args::Args;
use crate::config::load_config;
use crate::config::Config;
use crate::config::DEFAULT_CONFIG_PATH;
use crate::renderer::Renderer;

const CHAN_BUF_SIZE: usize = 1;

// TODO: add more modes and move this to a new module!
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Rolling,
    Power,
    Frequency,
}

impl DisplayMode {
    const ALL: [DisplayMode; 3] = [
        DisplayMode::Rolling,
        DisplayMode::Power,
        DisplayMode::Frequency,
    ];
}

impl Display for DisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DisplayMode::Rolling => "Rolling",
                DisplayMode::Power => "Power",
                DisplayMode::Frequency => "Frequency",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum GuiMessage {
    ModeSelected(DisplayMode),
    SliderUpdated((u32, SliderSide)),
    PointsUpdated(Vec<Vertex>),
    Tick(Instant),
    StopTx(sync::mpsc::Sender<()>),
    WindowClose(window::Id),
}

pub struct Gui {
    waveform: Waveform,
    selected_mode: Option<DisplayMode>,
    left_slider: u32,
    right_slider: u32,
    config: Config,
    update_vertices: Option<Vec<Vertex>>,
    gui_tx: Sender<GuiMessage>,
    gui_rx: Receiver<GuiMessage>,
    renderer_rx: Option<Receiver<GuiMessage>>,
    stop_tx: Option<sync::mpsc::Sender<()>>,
    display_buffer_tx: Sender<Array2<u8>>,
    display_buffer_rx: Receiver<Array2<u8>>,
}

impl Gui {
    fn check_update(&mut self) {
        match &self.update_vertices {
            Some(vertex_updates) => {
                self.waveform.update(vertex_updates.clone());
                self.update_vertices = None;
            }
            None => (),
        };
    }

    fn update_vertices(&mut self, new_vertices: Vec<Vertex>) {
        self.update_vertices = Some(new_vertices)
    }
}

impl Gui {
    fn new(config: Config) -> Self {
        let (gui_tx, gui_rx) = mpsc::channel::<GuiMessage>(CHAN_BUF_SIZE);
        let (display_buffer_tx, display_buffer_rx) = mpsc::channel::<Array2<u8>>(CHAN_BUF_SIZE);
        Self {
            waveform: Waveform::new(),
            selected_mode: Some(DisplayMode::Frequency),
            left_slider: config.left_slider_start,
            right_slider: config.right_slider_start,
            config,
            update_vertices: None,
            gui_tx,
            gui_rx,
            renderer_rx: None,
            stop_tx: None,
            display_buffer_rx,
            display_buffer_tx,
        }
    }

    pub fn update(&mut self, message: GuiMessage) -> Task<GuiMessage> {
        match message {
            GuiMessage::ModeSelected(mode) => {
                self.selected_mode = Some(mode);
                Task::none()
            }
            GuiMessage::SliderUpdated((value, SliderSide::Left)) => {
                self.left_slider = value;
                Task::none()
            }
            GuiMessage::SliderUpdated((value, SliderSide::Right)) => {
                self.right_slider = value;
                Task::none()
            }
            //TODO: figure out wtf is going on here, how can we do renders and vertex updates separately?
            GuiMessage::PointsUpdated(vertices) => {
                self.update_vertices(vertices);
                Task::none()
            }
            GuiMessage::Tick(_) => {
                self.check_update();
                Task::none()
            }
            GuiMessage::StopTx(tx) => {
                self.stop_tx = Some(tx);
                Task::none()
            }
            GuiMessage::WindowClose(id) => {
                self.stop_tx
                    .as_mut()
                    .expect("stop tx should be initialized by the time window close occurs")
                    .send(())
                    .expect("sending the stop signal expected to suceed on normal close");
                window::close::<GuiMessage>(id)
            }
        }
    }

    pub fn view(&self) -> iced::Element<GuiMessage> {
        let mode_select = pick_list(
            &DisplayMode::ALL[..],
            self.selected_mode,
            GuiMessage::ModeSelected,
        );

        let slider = DoubleSlider::new(
            self.config.min_freq_hz..=self.config.max_freq_hz,
            self.left_slider,
            self.right_slider,
            GuiMessage::SliderUpdated,
        );

        let controls_bar = row![
            horizontal_space().width(30),
            mode_select,
            slider,
            horizontal_space().width(30)
        ]
        .height(100)
        .align_y(Alignment::End)
        .spacing(10);

        let shader = shader(&self.waveform)
            .width(Length::Fill)
            .height(Length::Fill);

        let display_and_controls = column![shader, controls_bar].height(600).spacing(10);

        display_and_controls.into()
    }

    pub fn subscription(&self) -> iced::Subscription<GuiMessage> {
        Subscription::batch(vec![
            window::close_requests().map(GuiMessage::WindowClose),
            Subscription::run(audio_render_stream),
        ])
    }
}

impl Default for Gui {
    fn default() -> Self {
        let args = Args::parse();
        let mut config = load_config(&DEFAULT_CONFIG_PATH.to_string(), true);
        config.merge_with_args(args);
        Gui::new(config)
    }
}

fn audio_render_stream() -> impl Stream<Item = GuiMessage> {
    let (sender, receiver) = mpsc::channel(100);
    let renderer = Renderer::new(Config::default(), Some(sender)); // can we somehow pass the config in?
    thread::spawn(move || renderer.main_loop_external_updates());
    receiver
}
