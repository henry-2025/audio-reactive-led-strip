use std::fmt::Display;
mod double_slider;

use double_slider::DoubleSlider;
use iced::widget::horizontal_space;
use iced::widget::pick_list;
use iced::widget::row;
use iced::{Alignment, Sandbox};

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

#[derive(Debug, Clone, Copy)]
pub enum GuiMessage {
    ModeSelected(DisplayMode),
    LeftSlider(i32),
    RightSlider(i32),
}

pub struct Gui {
    selected_mode: Option<DisplayMode>,
    left_slider: i32,
    right_slider: i32,
}

impl Sandbox for Gui {
    type Message = GuiMessage;

    fn new() -> Self {
        Self {
            selected_mode: Some(DisplayMode::Frequency),
            left_slider: 0,
            right_slider: 0,
        }
    }

    fn title(&self) -> String {
        "Audio Reactive LED Strip".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            GuiMessage::ModeSelected(mode) => {
                self.selected_mode = Some(mode);
            }
            GuiMessage::LeftSlider(value) => self.left_slider = value,
            GuiMessage::RightSlider(value) => self.right_slider = value,
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let mode_select = pick_list(
            &DisplayMode::ALL[..],
            self.selected_mode,
            GuiMessage::ModeSelected,
        );

        let slider = DoubleSlider::new(
            0..=20000,
            self.left_slider,
            self.right_slider,
            GuiMessage::LeftSlider,
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

        controls_bar.into()
    }
}
