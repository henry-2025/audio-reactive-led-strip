use std::{
    fmt::{Debug, Display},
    thread::{self, Thread},
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
    future::select,
    select, SinkExt, StreamExt,
};

use crate::gui::GuiMessage;

const RENDERER_AUDIO_STREAM_START_TIMEOUT: Duration = Duration::from_secs(1);

pub struct AudioStream {
    gui_update_tx: Sender<GuiMessage>,
    audio_update_rx: Receiver<GuiMessage>,
    cpal_thread_rx: Option<Receiver<Vec<f32>>>,
    cpal_thread_handle: Option<thread::JoinHandle<()>>,
    cpal_device: cpal::Device,
    cpal_config: cpal::SupportedStreamConfig,
    frame_duration: Duration,
    mic_rate: u32,
}

#[derive(Clone)]
pub struct RecordingDevice(cpal::Device);
impl RecordingDevice {
    pub fn new(device: cpal::Device) -> Self {
        Self(device)
    }
}

struct CpalStream {
    stream_tx: Sender<Vec<f32>>,
    last_frame_capture: Instant,
    frame_duration: Duration,
    cpal_device: cpal::Device,
    cpal_config: cpal::SupportedStreamConfig,
}

impl AudioStream {
    pub fn new(
        mut gui_update_tx: futures::channel::mpsc::Sender<GuiMessage>,
        update_fps: u32,
        mic_rate: u32,
        audio_device: RecordingDevice,
    ) -> Self {
        let audio_config = get_audio_configs(&audio_device.0, mic_rate).expect("default audio device could not be configured")[0].clone();
        let (audio_update_tx, audio_update_rx) = channel(1);
        gui_update_tx
            .try_send(GuiMessage::AudioCaptureTx(audio_update_tx))
            .expect("gui update tx should be open when the audio stream is created");
        Self {
            gui_update_tx,
            audio_update_rx,
            cpal_thread_rx: None,
            cpal_thread_handle: None,
            cpal_device: audio_device.0,
            cpal_config: audio_config,
            frame_duration: Duration::from_secs_f64(1. / update_fps as f64),
            mic_rate,
        }
    }

    pub async fn start(&mut self) {
        self.start_cpal_stream(self.cpal_device.clone());

        loop {
            select! {
                gui_update = self.audio_update_rx.select_next_some() => match gui_update {
                    GuiMessage::RecordingDeviceSelected(recording_device) => {
                        if let Err(err) = self.start_cpal_stream(recording_device.0) {
                            println!("Encountered an error in device selection: {}. Select another audio device", err);
                        }
                    },
                    default => println!("received message {:?} but thread does not know how to handle this", default),
                },
                updated_points = self.cpal_thread_rx.as_mut().expect("should be set at the time of start").select_next_some() => {
                    let _ = self.gui_update_tx.send(GuiMessage::AudioBuffer(updated_points)).await;
                },
            }
        }
    }

    fn maybe_shutdown_stream(&mut self) {
        // let the audio capture thread terminate if it is already running
        self.cpal_thread_handle.as_ref().map(|x| {
            x.thread().unpark();
            self.cpal_thread_rx = None;
        });
    }

    fn start_cpal_stream(&mut self, cpal_device: cpal::Device) -> Result<(), NoDevicesError> {
        if !(self.cpal_thread_handle.is_none() == self.cpal_thread_rx.is_none()) {
            panic!("these two should either both be set or both empty");
        }

        self.maybe_shutdown_stream();
        self.cpal_config = get_audio_configs(&cpal_device, self.mic_rate)?[0].clone();
        self.cpal_device = cpal_device.clone();

        let (tx, rx) = channel(1);
        self.cpal_thread_rx = Some(rx);
        let cpal_stream = CpalStream::new(
            tx,
            cpal_device,
            self.cpal_config.clone(),
            self.frame_duration,
        );
        self.cpal_thread_handle = Some(cpal_stream.start());
        Ok(())
    }
}

impl CpalStream {
    pub fn new(
        stream_tx: Sender<Vec<f32>>,
        cpal_device: cpal::Device,
        cpal_config: cpal::SupportedStreamConfig,
        frame_duration: Duration,
    ) -> Self {
        Self {
            stream_tx,
            cpal_device,
            cpal_config,
            last_frame_capture: Instant::now(),
            frame_duration,
        }
    }

    pub fn start(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let stream = self.build_input_stream();
            stream.play().expect("error playing audio stream");
            thread::park();
        })
    }

    fn build_input_stream(mut self) -> cpal::Stream {
        self.cpal_device
            .clone()
            .build_input_stream(
                &self.cpal_config.config(),
                move |audio_data: &[f32], _: &InputCallbackInfo| {
                    self.send_buffer_to_audio_stream(audio_data);
                },
                |e: StreamError| {
                    println!("Error received from input stream: {}", e);
                },
                Some(RENDERER_AUDIO_STREAM_START_TIMEOUT),
            )
            .expect("Could not build audio stream")
    }

    fn send_buffer_to_audio_stream(&mut self, audio_data: &[f32]) {
        if self.last_frame_capture.elapsed() > self.frame_duration {
            self.last_frame_capture = Instant::now();
            self.stream_tx
                .try_send(audio_data.to_vec())
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

#[derive(Debug)]
struct NoDevicesError(String);

impl Display for NoDevicesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn get_audio_configs(
    device: &cpal::Device,
    mic_rate: u32,
) -> Result<Vec<SupportedStreamConfig>, NoDevicesError> {
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
        Err(NoDevicesError(format!(
            "Could not create the intended audio input config: 1 channel, {}Hz, f32 format",
            mic_rate
        )))
    } else {
        Ok(configs)
    }
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
