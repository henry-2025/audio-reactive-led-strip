use std::{
    fmt::{Debug, Display},
    thread::{self},
    time::{Duration, Instant},
};

use cpal::{
    default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, SampleFormat, SampleRate, StreamError, SupportedStreamConfig,
};
use iced::futures::{
    self,
    channel::mpsc::{channel, Receiver, Sender},
    select, SinkExt, StreamExt,
};

use crate::{config::Config, gui::GuiMessage};

const RENDERER_AUDIO_STREAM_START_TIMEOUT: Duration = Duration::from_secs(1);

pub struct AudioStreamManager {
    gui_update_tx: Sender<GuiMessage>,
    audio_update_rx: Receiver<GuiMessage>,
    cpal_thread_rx: Option<Receiver<Channels>>,
    cpal_thread_handle: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct RecordingDevice {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    frame_duration: Duration,
}

impl RecordingDevice {
    pub fn new(device: cpal::Device, frame_duration: Duration, mic_rate: u32) -> Option<Self> {
        let config = get_audio_config(&device, Some(mic_rate))
            .or(get_audio_config(&device, None))
            .ok()?;
        Some(Self {
            config,
            device,
            frame_duration,
        })
    }
}

struct CpalStream {
    stream_tx: Sender<Channels>,
    last_frame_capture: Instant,
    recording_device: RecordingDevice,
}

#[derive(Debug, Clone)]
struct Channels(Vec<f32>, Option<Vec<f32>>);

impl AudioStreamManager {
    pub fn new(mut gui_update_tx: futures::channel::mpsc::Sender<GuiMessage>) -> Self {
        let (audio_update_tx, audio_update_rx) = channel(1);
        gui_update_tx
            .try_send(GuiMessage::AudioCaptureTx(audio_update_tx))
            .expect("gui update tx should be open when the audio stream is created");
        Self {
            gui_update_tx,
            audio_update_rx,
            cpal_thread_rx: None,
            cpal_thread_handle: None,
        }
    }

    pub async fn start(&mut self) {
        self.init_render_thread().await;
        loop {
            select! {
                gui_update = self.audio_update_rx.select_next_some() => match gui_update {
                    GuiMessage::RecordingDeviceSelected(recording_device) => self.start_cpal_stream(recording_device),
                    default => println!("render thread loop received message {:?} but thread does not know how to handle this", default),
                },
                updated_points = self.cpal_thread_rx.as_mut().expect("should be set at the time of start").select_next_some() => {
                    let _ = self.gui_update_tx.send(GuiMessage::AudioBuffer(updated_points)).await;
                },
            }
        }
    }

    async fn init_render_thread(&mut self) {
        while let Some(message) = self.audio_update_rx.next().await {
            match message {
                GuiMessage::RecordingDeviceSelected(recording_device) => {
                    self.start_cpal_stream(recording_device);
                    break;
                }
                    default => println!("render thread init received message {:?} but thread does not know how to handle this", default),
            }
        }
    }

    fn maybe_shutdown_stream(&mut self) {
        // let the audio capture thread terminate if it is already running
        self.cpal_thread_handle.as_ref().map(|handle| {
            handle.thread().unpark();
        });
    }

    fn start_cpal_stream(&mut self, recording_device: RecordingDevice) {
        if !(self.cpal_thread_handle.is_none() == self.cpal_thread_rx.is_none()) {
            panic!("these two should either both be set or both empty");
        }

        self.maybe_shutdown_stream();

        let (tx, rx) = channel(1);
        self.cpal_thread_rx = Some(rx);
        let cpal_stream = CpalStream::new(tx, recording_device);
        self.cpal_thread_handle = Some(cpal_stream.start());
    }
}

impl CpalStream {
    pub fn new(stream_tx: Sender<Channels>, recording_device: RecordingDevice) -> Self {
        Self {
            stream_tx,
            recording_device,
            last_frame_capture: Instant::now(),
        }
    }

    pub fn start(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let stream = self.build_input_stream();
            stream.play().expect("error playing audio stream");
            thread::park();
            stream.pause().expect("could not pause audio stream");
        })
    }

    fn build_input_stream(mut self) -> cpal::Stream {
        self.recording_device
            .device
            .clone()
            .build_input_stream(
                &self.recording_device.config.config(),
                move |audio_data: &[f32], _: &InputCallbackInfo| {
                    self.capture_audio_frame(audio_data);
                },
                |e: StreamError| {
                    println!("Error received from input stream: {}", e);
                },
                Some(RENDERER_AUDIO_STREAM_START_TIMEOUT),
            )
            .expect("Could not build audio stream")
    }

    fn capture_audio_frame(&mut self, audio_data: &[f32]) {
        if self.last_frame_capture.elapsed() > self.recording_device.frame_duration {
            self.last_frame_capture = Instant::now();

            self.stream_tx
                .try_send(self.get_channel_audio(audio_data))
                .expect("should be able to send audio data back to gui");
        }
    }

    fn get_channel_audio(&self, audio_data: &[f32]) -> Channels {
        if self.recording_device.config.config().channels == 2 {
            Channels(
                audio_data.iter().cloned().step_by(2).collect(),
                Some(audio_data[1..].iter().cloned().step_by(2).collect()),
            )
        } else {
            Channels(audio_data.to_vec(), None)
        }
    }
}

pub fn get_recording_devices(config: &Config) -> Vec<RecordingDevice> {
    default_host()
        .input_devices()
        .expect("system must have at least one recording device")
        .map(|device| {
            RecordingDevice::new(
                device,
                Duration::from_secs_f64(1.0 / config.fps as f64),
                config.mic_rate,
            )
        })
        .flatten()
        .collect()
}

pub fn get_default_recording_device(config: &Config) -> RecordingDevice {
    let device = default_host()
        .default_input_device()
        .expect("system must have at least one recording device");

    RecordingDevice::new(
        device,
        Duration::from_secs_f64(1.0 / config.fps as f64),
        config.mic_rate,
    )
    .expect("default recording device could not be configured")
}

#[derive(Debug)]
struct NoDevicesError(String);

impl Display for NoDevicesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn get_audio_config(
    device: &cpal::Device,
    mic_rate: Option<u32>,
) -> Result<SupportedStreamConfig, NoDevicesError> {
    let configs: Vec<SupportedStreamConfig> = device
        .supported_input_configs()
        .expect("device should have supported input configs")
        .filter_map(|x| {
            let sample_rates: SupportedStreamConfig;

            if let Some(rate) = mic_rate {
                if x.max_sample_rate() < SampleRate(rate) || x.min_sample_rate() > SampleRate(rate)
                {
                    return None;
                }
                sample_rates = x.with_sample_rate(SampleRate(rate));
            } else {
                sample_rates = x.with_max_sample_rate();
            }
            //TODO: for now, mac only supports floating-point sampling formats. In the future,
            //will want to compile to support i16 and u16 formats as well. Will be a good
            //case for pattern matching
            if sample_rates.sample_format() == SampleFormat::F32 && sample_rates.channels() <= 2 {
                Some(sample_rates)
            } else {
                None
            }
        })
        .collect();

    if configs.is_empty() {
        if let Some(rate) = mic_rate {
            Err(NoDevicesError(format!(
                "Could not create the intended audio input config: 1 or 2 channels, {}Hz, f32 format",
                rate
            )))
        } else {
            Err(NoDevicesError(
                "Could not create the any audio input config: 1 or 2 channels, f32 format"
                    .to_string(),
            ))
        }
    } else {
        Ok(configs[0].clone())
    }
}

impl Display for RecordingDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}, {} Hz",
            self.device
                .name()
                .expect("should have a name for this device"),
            self.config.sample_rate().0
        )
    }
}

impl Debug for RecordingDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl PartialEq for RecordingDevice {
    fn eq(&self, other: &Self) -> bool {
        self.device
            .name()
            .expect("this device should have a readable name")
            < other
                .device
                .name()
                .expect("the compared to device should have a readable name")
    }
}
