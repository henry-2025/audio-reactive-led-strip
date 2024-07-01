use iced::Sandbox;

struct ControlBar {
    frequency_slider: FrequencySlider,
    mode_selection: ModeSelection,
}

struct FrequencySlider {
    left_bound: u32,
    right_bound: u32,
}

struct ModeSelection {
    mode: DisplayMode,
}

// TODO: add more modes and move this to a new module!
enum DisplayMode {
    Rolling,
    Power,
    Frequency,
}

#[derive(Debug, Clone, Copy)]
enum ModeSelected {
    Selected,
}
//impl Sandbox for ModeSelection {
//    type Message = ModeSelected;
//
//    fn new() -> Self {
//        Self {
//            mode: DisplayMode::Frequency,
//        }
//    }
//}
//
