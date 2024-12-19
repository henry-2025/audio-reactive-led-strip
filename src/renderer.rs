use std::{
    io::{self, Read},
    sync, thread,
    time::{Duration, Instant},
};

use cpal::{
    default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, SampleFormat, SampleRate, StreamError, SupportedStreamConfig,
};
use glam::Vec3;
use iced::futures::{self, executor::block_on, SinkExt, StreamExt};
use ndarray::{arr1, concatenate, s, Array1, Array2, Axis};

use crate::{
    config::Config,
    dsp::{self, Dsp, Preset},
    gui::{waveform::Point, GuiMessage},
    led::ESP8266Conn,
};

const RENDERER_AUDIO_STREAM_START_TIMEOUT: Duration = Duration::from_secs(1);

pub struct Renderer {
    gui_update_tx: Option<futures::channel::mpsc::Sender<GuiMessage>>,
    state: RendererState,
}

struct RendererReady {
    selected_preset: dsp::Preset,
    config: Config,
    frame_duration: Duration,
    conn: ESP8266Conn,
    dsp: Dsp,
    last_render: Instant,
    rolling_history: Array1<f64>,
    display_values: Array2<f64>,
    send_buffer: Array2<u8>,
    ignore_io_errors: bool,
}

enum RendererState {
    Pending,
    Ready(RendererReady),
}
impl RendererState {
    fn new(config: Config) -> Self {
        RendererState::Ready(RendererReady::new(config))
    }
}

impl RendererReady {
    fn new(config: Config) -> RendererReady {
        Self {
            display_values: Array2::<f64>::zeros((config.n_points as usize, 3)),
            send_buffer: Array2::<u8>::zeros((config.n_points as usize, 3)),
            selected_preset: Preset::Scroll,
            rolling_history: Array1::<f64>::zeros(config.n_fft_bins as usize),
            frame_duration: Duration::from_secs_f64(1. / config.fps as f64),
            last_render: Instant::now(), // start rendering on our first sample
            conn: ESP8266Conn::new(&config).expect("esp8266 connection should have been made"),
            dsp: Dsp::new(config.clone()),
            config,
            ignore_io_errors: false,
        }
    }
}

impl Renderer {
    pub fn new(
        update_tx: Option<futures::channel::mpsc::Sender<GuiMessage>>,
        config: Option<Config>,
    ) -> Self {
        match update_tx {
            Some(_) => {
                assert!(config.is_none(), "expected config to be none when an update tx was passed (this is the behavior of gui mode)");
                Self {
                    gui_update_tx: update_tx,
                    state: RendererState::Pending,
                }
            }
            None => match config {
                Some(c) => Self {
                    gui_update_tx: None,
                    state: RendererState::Ready(RendererReady::new(c)),
                },
                None =>
                    panic!("expected config to exist when the update tx is set to none (this is the behavior of cli mode)")
            },
        }
    }

    // start the main loop with an update message channel
    pub fn main_loop(self, renderer_update_rx: futures::channel::mpsc::Receiver<GuiMessage>) {
        self.start_audio_stream(renderer_update_rx);
        thread::park();
    }

    // Start the audio stream with a channel that can receive updates from the GUI
    fn start_audio_stream(mut self, mut update_rx: futures::channel::mpsc::Receiver<GuiMessage>) {
        while let RendererState::Pending = self.state {
            let message = block_on(update_rx.next());
            if let Some(m) = message {
                match m {
                    GuiMessage::Config(config) => self.state = RendererState::new(config),
                    _ => {}
                }
            } else {
                panic!("gui stream closed before audio stream could initialize")
            }
        }
        thread::spawn(move || {
            // wait for stream to initialize
            if let RendererState::Ready(ready) = &self.state {
                let device = default_host()
                    .default_input_device()
                    .expect("No default input device could be bound");

                let configs: Vec<SupportedStreamConfig> = device
                    .supported_input_configs()
                    .unwrap()
                    .filter_map(|x| {
                        if x.max_sample_rate() < SampleRate(ready.config.mic_rate)
                            || x.min_sample_rate() > SampleRate(ready.config.mic_rate)
                        {
                            None
                        } else {
                            let sample_rates =
                                x.with_sample_rate(SampleRate(ready.config.mic_rate));
                            //TODO: for now, mac only supports floating-point sampling formats. In the future,
                            //will want to compile to support i16 and u16 formats as well. Will be a good
                            //case for pattern matching
                            if sample_rates.sample_format() == SampleFormat::F32
                                && sample_rates.channels() == 1
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
                        ready.config.mic_rate
                    );
                }
                let stream = device
                    .build_input_stream(
                        &configs[0].config(),
                        move |audio_data: &[f32], _: &InputCallbackInfo| {
                            self.update(audio_data);
                        },
                        |e: StreamError| {
                            println!("Error received from input stream: {}", e);
                        },
                        Some(RENDERER_AUDIO_STREAM_START_TIMEOUT),
                    )
                    .expect("Could not build audio stream");

                stream.play().expect("error playing audio stream");
                thread::park();
            } else {
                panic!("renderer state was not initialized but should have been");
            }
        });
    }

    fn update(&mut self, audio_data: &[f32]) {
        // move in new audio samples to buffer (back is newest)
        let new_data = arr1(audio_data).mapv(f64::from);
        if let RendererState::Ready(ready) = &mut self.state {
            ready.rolling_history = concatenate![
                Axis(0),
                ready.rolling_history.slice(s![new_data.shape()[0]..]),
                new_data
            ];

            // re-render when we encounter a frame boundary
            if ready.last_render.elapsed() > ready.frame_duration {
                ready.last_render = Instant::now();

                // transform the audio to the frequency space and then to the mel spectrum
                let audio_data_rfft = ready.dsp.exec_rfft(&ready.rolling_history);
                let mut audio_data_mel = ready.dsp.get_mel_repr(&audio_data_rfft);
                ready.dsp.gain_and_smooth(&mut audio_data_mel);

                ready.dsp.apply_transform_inplace(
                    ready.selected_preset.clone(),
                    &mut ready.display_values,
                );

                let mut new_send_buffer: Array2<u8> = ready.display_values.map(|v| {
                    if *v < 0.0 {
                        0
                    } else if *v > 255.0 {
                        255
                    } else {
                        *v as u8
                    }
                });

                let io_result = ready.conn.update(&mut new_send_buffer, &ready.send_buffer);

                if let Err(error) = io_result {
                    if !ready.ignore_io_errors {
                        if error.kind() == std::io::ErrorKind::HostUnreachable
                            || error.kind() == std::io::ErrorKind::NetworkUnreachable
                        {
                            print!("Encountered an IO error {} press ENTER to ignore or any character + ENTER to quit", error.to_string());
                            let mut input = String::new();
                            io::stdin()
                                .read_line(&mut input)
                                .expect("unable to read input");
                            if input.len() > 1 {
                                self.gui_update_tx
                                    .as_mut()
                                    .expect("should be able to acquire mutable gui update tx")
                                    .try_send(GuiMessage::RendererStop)
                                    .expect("should be able to send a quit signal to gui");
                            } else {
                                println!("Ignoring io errors for now");
                                ready.ignore_io_errors = true;
                            }
                        } else {
                            panic!("unexpected io error encountered when writing to leds");
                        }
                    }
                }

                self.gui_update_tx
                    .as_mut()
                    .expect("should be able to acquire mutable gui update tx")
                    .try_send(GuiMessage::PointsUpdated(send_buffer_to_points(
                        &new_send_buffer,
                    )))
                    .expect("should be able to send the updated points buffer to the gui");

                ready.send_buffer = new_send_buffer;
            }
        }
    }

    pub fn main_loop_with_external_updates(mut self) {
        // create the stop channels in here and then send them to the gui
        let (renderer_input_tx, renderer_input_rx) =
            futures::channel::mpsc::channel::<GuiMessage>(1);

        let gui_channels_flush_result = async {
            let gui_update_tx = self
                .gui_update_tx
                .as_mut()
                .expect("update tx should exist in mail loop setup");
            gui_update_tx
                .feed(GuiMessage::UpdateTx(renderer_input_tx))
                .await
                .expect("send the renderer's input tx to gui sender unsuccesful");
            gui_update_tx
                .feed(GuiMessage::RendererThread(thread::current()))
                .await
                .expect("send the renderer's handle to gui sender unsuccesful");

            gui_update_tx.flush().await
        };

        block_on(gui_channels_flush_result)
            .expect("flushing the renderer's stop tx and input tx to gui sender unsuccessful");

        self.main_loop(renderer_input_rx);
    }
}

fn send_buffer_to_points(send_buffer: &Array2<u8>) -> Vec<Point> {
    send_buffer
        .axis_iter(Axis(0))
        .map(|col| Point {
            color: Vec3::new(
                col[0] as f32 / 255.0,
                col[1] as f32 / 255.0,
                col[2] as f32 / 255.0,
            ),
        })
        .collect()
}
