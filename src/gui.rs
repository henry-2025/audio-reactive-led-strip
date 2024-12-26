mod double_slider;
pub mod waveform;

use clap::Parser;
use double_slider::{DoubleSlider, SliderSide};
use iced::{
    futures::{channel::mpsc::Sender, Stream},
    stream::channel,
    widget::{column, horizontal_space, pick_list, row, shader},
    window, Alignment, Length, Subscription, Task,
};
use std::io;
use waveform::Waveform;
use waveform::WaveformDisplayMode;

use crate::{
    args::Args,
    audio::{self, RecordingDevice},
    dsp::Preset,
};
use crate::{
    audio::AudioStreamManager,
    config::{load_config, Config, DEFAULT_CONFIG_PATH},
    dsp::Dsp,
    led::ESP8266Conn,
};

#[derive(Clone, Debug)]
pub enum GuiMessage {
    PresetSelected(Preset),
    WindowClose(window::Id),
    WaveformDisplayModeSelected(WaveformDisplayMode),
    AudioCaptureTx(Sender<GuiMessage>),
    RecordingDeviceSelected(RecordingDevice),
    SliderUpdated((u32, SliderSide)),
    AudioBuffer(Vec<f32>),
}

pub struct Gui {
    waveform: Waveform,
    selected_preset: Option<Preset>,
    selected_waveform_display: Option<WaveformDisplayMode>,
    left_slider: u32,
    right_slider: u32,
    config: Config,
    dsp: Dsp,
    esp_device: ESP8266Conn,
    recording_device: Option<RecordingDevice>,
    all_recording_devices: Vec<RecordingDevice>,
    audio_capture_tx: Option<Sender<GuiMessage>>,
    ignore_io_errors: bool,
}

impl Gui {
    fn new(config: Config) -> Self {
        let waveform_display_mode = WaveformDisplayMode::Colors;
        Self {
            waveform: Waveform::new(waveform_display_mode),
            selected_preset: Some(Preset::DEFAULT),
            selected_waveform_display: Some(WaveformDisplayMode::DEFAULT),
            left_slider: config.left_slider_start,
            right_slider: config.right_slider_start,
            dsp: Dsp::new(&config),
            esp_device: ESP8266Conn::new(&config)
                .expect("esp8266 connection should have been made"),
            recording_device: Some(audio::get_default_recording_device(&config)),
            all_recording_devices: audio::get_recording_devices(&config),
            config,
            audio_capture_tx: None,
            ignore_io_errors: false,
        }
    }

    fn close_and_maybe_stop_render_thread(&self, id: window::Id) -> Task<GuiMessage> {
        window::close::<GuiMessage>(id)
    }

    pub fn update(&mut self, message: GuiMessage) -> Task<GuiMessage> {
        match message {
            GuiMessage::PresetSelected(mode) => {
                self.selected_preset = Some(mode);
                self.dsp.selected_preset = mode;
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
            GuiMessage::WindowClose(id) => self.close_and_maybe_stop_render_thread(id),
            GuiMessage::WaveformDisplayModeSelected(mode) => {
                self.selected_waveform_display = Some(mode);
                self.waveform.set_mode(mode);
                Task::none()
            }
            GuiMessage::AudioBuffer(vec) => {
                self.dsp.update_audio(&vec);
                let device_buffer = self.dsp.get_send_buffer();
                let mel_display = self.dsp.get_current_mel_display();

                self.waveform.update_points(&device_buffer);
                self.waveform.update_mel_display(mel_display);

                self.esp_device
                    .send_buffer_to_device(&device_buffer)
                    .or_else(|err| self.handle_io_errors(err))
                    .or_else(|_| self.error_handle_prompt())
                    .expect("at this point should have addressed all errors");

                Task::none()
            }
            GuiMessage::RecordingDeviceSelected(recording_device) => {
                self.recording_device = Some(recording_device.clone());
                self.send_to_audio_capture_thread(GuiMessage::RecordingDeviceSelected(
                    recording_device,
                ));
                Task::none()
            }
            GuiMessage::AudioCaptureTx(mut tx) => {
                tx.try_send(GuiMessage::RecordingDeviceSelected(
                    self.recording_device.as_ref().unwrap().clone(),
                ))
                .expect("want to send recording device to render thread");
                self.audio_capture_tx = Some(tx);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<GuiMessage> {
        let mode_select = pick_list(
            &Preset::ALL[..],
            self.selected_preset,
            GuiMessage::PresetSelected,
        );

        let audio_device_select = pick_list(
            self.all_recording_devices.as_slice(),
            self.recording_device.clone(),
            GuiMessage::RecordingDeviceSelected,
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
            audio_device_select,
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
            Subscription::run_with_id(1, Self::start_new_audio_stream()),
        ])
    }

    fn start_new_audio_stream() -> impl Stream<Item = GuiMessage> {
        channel(1, move |audio_tx| async move {
            let mut audio_stream: AudioStreamManager = AudioStreamManager::new(audio_tx);
            audio_stream.start().await;
        })
    }

    fn handle_io_errors(&self, err: io::Error) -> Result<usize, io::Error> {
        if !self.ignore_io_errors
            && (err.kind() == std::io::ErrorKind::HostUnreachable
                || err.kind() == std::io::ErrorKind::NetworkUnreachable)
        {
            Err(err)
        } else {
            Ok(0)
        }
    }

    fn error_handle_prompt(&mut self) -> Result<usize, io::Error> {
        println!("encountered unhandled io error and handling prompt");
        self.ignore_io_errors = true;
        Ok(0)
    }

    fn send_to_audio_capture_thread(&mut self, message: GuiMessage) {
        self.audio_capture_tx
            .as_mut()
            .expect("audio capture tx should be set on this call")
            .try_send(message)
            .expect("message send to audio capture should succeed");
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
