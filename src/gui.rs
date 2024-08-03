mod double_slider;
mod waveform;

use double_slider::DoubleSlider;
use double_slider::SliderSide;
pub use iced::application::Application;
use iced::widget::column;
use iced::widget::horizontal_space;
use iced::widget::pick_list;
use iced::widget::row;
use iced::widget::shader;
use iced::window;
use iced::Alignment;
use iced::Command;
use iced::Length;
use std::fmt::Display;
use std::time::Instant;
use waveform::pipeline::Vertex;
use waveform::Waveform;

use crate::config::Config;

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
}

pub struct Gui {
    waveform: Waveform,
    selected_mode: Option<DisplayMode>,
    left_slider: u32,
    right_slider: u32,
    config: Config,
    update_vertices: Option<Vec<Vertex>>,
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

impl Application for Gui {
    type Executor = iced::executor::Default;

    type Message = GuiMessage;

    type Theme = iced_style::Theme;

    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                waveform: Waveform::new(),
                selected_mode: Some(DisplayMode::Frequency),
                left_slider: flags.config.left_slider_start,
                right_slider: flags.config.right_slider_start,
                config: flags.config,
                update_vertices: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Audio Reactive LED Strip".to_string()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<GuiMessage> {
        match message {
            GuiMessage::ModeSelected(mode) => {
                self.selected_mode = Some(mode);
            }
            GuiMessage::SliderUpdated((value, SliderSide::Left)) => {
                self.left_slider = value;
            }
            GuiMessage::SliderUpdated((value, SliderSide::Right)) => {
                self.right_slider = value;
            }
            //TODO: figure out wtf is going on here, how can we do renders and vertex updates separately?
            GuiMessage::PointsUpdated(vertices) => self.update_vertices(vertices),
            GuiMessage::Tick(_) => self.check_update(),
        };
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let mode_select = pick_list(
            &DisplayMode::ALL[..],
            self.selected_mode,
            GuiMessage::ModeSelected,
        );

        let slider = DoubleSlider::new(
            self.config.min_frequency..=self.config.max_frequency,
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
        .align_items(Alignment::End)
        .spacing(10);

        let shader = shader(&self.waveform)
            .width(Length::Fill)
            .height(Length::Fill);

        let display_and_controls = column![shader, controls_bar].height(600).spacing(10);

        display_and_controls.into()
    }

    fn theme(&self) -> Self::Theme {
        Self::Theme::default()
    }

    fn style(&self) -> <Self::Theme as iced::application::StyleSheet>::Style {
        <Self::Theme as iced::application::StyleSheet>::Style::default()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        window::frames().map(Self::Message::Tick)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

pub struct Flags {
    config: Config,
}

impl Into<Flags> for Config {
    fn into(self) -> Flags {
        Flags { config: self }
    }
}
