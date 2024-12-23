use std::{
    fmt::{Debug, Display},
    thread,
    time::{Duration, Instant},
};

use cpal::{
    default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, SampleFormat, SampleRate, StreamError, SupportedStreamConfig,
};
use iced::futures::{self, channel::mpsc::Sender};

use crate::gui::{waveform, GuiMessage};

const RENDERER_AUDIO_STREAM_START_TIMEOUT: Duration = Duration::from_secs(1);

pub struct AudioStream {
    gui_update_tx: Sender<GuiMessage>,
    audio_device: cpal::Device,
    audio_config: cpal::SupportedStreamConfig,
    frame_duration: Duration,
    last_frame_capture: Instant,
}

#[derive(Clone)]
pub struct RecordingDevice(cpal::Device);
impl RecordingDevice {
    pub fn new(device: cpal::Device) -> Self {
        Self(device)
    }
}

impl AudioStream {
    pub fn new(
        gui_update_tx: futures::channel::mpsc::Sender<GuiMessage>,
        update_fps: u32,
        mic_rate: u32,
        audio_device: RecordingDevice,
    ) -> Self {
        let audio_config = get_audio_configs(&audio_device.0, mic_rate)[0].clone();
        Self {
            gui_update_tx,
            audio_device: audio_device.0,
            audio_config,
            frame_duration: Duration::from_secs_f64(1. / update_fps as f64),
            last_frame_capture: Instant::now(),
        }
    }

    pub fn start(mut self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            self.gui_update_tx
                .try_send(GuiMessage::AudioCaptureThread(thread::current()))
                .expect("should be able to send current thread handle back to main");
            let stream = self.build_input_stream();
            stream.play().expect("error playing audio stream");
            thread::park();
        })
    }

    fn build_input_stream(mut self) -> cpal::Stream {
        self.audio_device
            .clone()
            .build_input_stream(
                &self.audio_config.config(),
                move |audio_data: &[f32], _: &InputCallbackInfo| {
                    self.send_buffer_to_gui(audio_data);
                },
                |e: StreamError| {
                    println!("Error received from input stream: {}", e);
                },
                Some(RENDERER_AUDIO_STREAM_START_TIMEOUT),
            )
            .expect("Could not build audio stream")
    }

    fn send_buffer_to_gui(&mut self, audio_data: &[f32]) {
        if self.last_frame_capture.elapsed() > self.frame_duration {
            self.last_frame_capture = Instant::now();
            self.gui_update_tx
                .try_send(GuiMessage::AudioBuffer(audio_data.to_vec()))
                .expect("should be able to send audio data back to gui");
        }
    }
}

pub fn get_recording_devices() -> Vec<RecordingDevice> {
    default_host()
        .input_devices()
        .expect("should have cpal devices")
        .map(RecordingDevice::new)
        .collect()
}

fn get_audio_configs(device: &cpal::Device, mic_rate: u32) -> Vec<SupportedStreamConfig> {
    let configs: Vec<SupportedStreamConfig> = device
        .supported_input_configs()
        .unwrap()
        .filter_map(|x| {
            if x.max_sample_rate() < SampleRate(mic_rate)
                || x.min_sample_rate() > SampleRate(mic_rate)
            {
                None
            } else {
                let sample_rates = x.with_sample_rate(SampleRate(mic_rate));
                //TODO: for now, mac only supports floating-point sampling formats. In the future,
                //will want to compile to support i16 and u16 formats as well. Will be a good
                //case for pattern matching
                if sample_rates.sample_format() == SampleFormat::F32 && sample_rates.channels() == 1
                {
                    Some(sample_rates)
                } else {
                    None
                }
            }
        })
        .collect();
    if configs.is_empty() {
        panic!(
            "Could not create the intended audio input config: 1 channel, {}Hz, f32 format",
            mic_rate
        );
    }
    configs
}

impl Display for RecordingDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.name().expect("should have a name for this device")
        )
    }
}

impl Debug for RecordingDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.name().expect("should have a name for this device")
        )
    }
}

impl PartialEq for RecordingDevice {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .name()
            .expect("this device should have a readable name")
            < other
                .0
                .name()
                .expect("the compared to device should have a readable name")
    }
}
