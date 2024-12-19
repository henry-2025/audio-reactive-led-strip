mod double_slider;
pub mod waveform;

use clap::Parser;
use double_slider::{DoubleSlider, SliderSide};
use iced::{
    futures::{self, channel::mpsc::channel, Stream},
    widget::{column, horizontal_space, pick_list, row, shader},
    window::{self, get_oldest},
    Alignment, Length, Subscription, Task,
};
use std::{sync, thread};
use waveform::Waveform;
use waveform::{Point, WaveformDisplayMode};

use crate::config::{load_config, Config, DEFAULT_CONFIG_PATH};
use crate::renderer::Renderer;
use crate::{args::Args, dsp::Preset};

#[derive(Debug, Clone)]
pub enum GuiMessage {
    ModeSelected(Preset),
    WaveformDisplayModeSelected(WaveformDisplayMode),
    SliderUpdated((u32, SliderSide)),
    PointsUpdated(Vec<Point>),
    StopTx(std::sync::mpsc::Sender<()>),
    UpdateTx(futures::channel::mpsc::Sender<GuiMessage>),
    Config(Config),
    WindowClose(window::Id),
    RendererStop,
}

pub struct Gui {
    waveform: Waveform,
    selected_mode: Option<Preset>,
    selected_waveform_display: Option<WaveformDisplayMode>,
    left_slider: u32,
    right_slider: u32,
    config: Config,
    gui_tx: Option<futures::channel::mpsc::Sender<GuiMessage>>,
    stop_tx: Option<sync::mpsc::Sender<()>>,
}

impl Gui {
    fn new(config: Config) -> Self {
        let waveform_display_mode = WaveformDisplayMode::Colors;
        Self {
            waveform: Waveform::new(waveform_display_mode),
            selected_mode: Some(Preset::Spectrum),
            selected_waveform_display: Some(waveform_display_mode),
            left_slider: config.left_slider_start,
            right_slider: config.right_slider_start,
            config,
            gui_tx: None,
            stop_tx: None,
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
            GuiMessage::PointsUpdated(vertices) => {
                self.waveform.update_points(vertices);
                Task::none()
            }
            GuiMessage::StopTx(renderer_stop_tx) => {
                self.stop_tx = Some(renderer_stop_tx);
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
            GuiMessage::RendererStop => get_oldest().and_then(window::close),
            GuiMessage::WaveformDisplayModeSelected(mode) => {
                self.selected_waveform_display = Some(mode);
                self.waveform.set_mode(mode);
                Task::none()
            }
            GuiMessage::Config(_) => Task::none(),
            GuiMessage::UpdateTx(mut renderer_update_tx) => {
                renderer_update_tx
                    .try_send(GuiMessage::Config(self.config.clone()))
                    .expect("gui update input channel should be open at this call");
                self.gui_tx = Some(renderer_update_tx);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<GuiMessage> {
        let mode_select = pick_list(
            &Preset::ALL[..],
            self.selected_mode,
            GuiMessage::ModeSelected,
        );

        let waveform_select = pick_list(
            &WaveformDisplayMode::ALL[..],
            self.selected_waveform_display,
            GuiMessage::WaveformDisplayModeSelected,
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
            waveform_select,
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
    let (sender, receiver) = channel(1);
    let renderer = Renderer::new(Some(sender), None);
    thread::spawn(move || renderer.main_loop_with_external_updates());
    receiver
}
