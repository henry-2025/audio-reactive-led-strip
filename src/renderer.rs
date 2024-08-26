use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cpal::{traits::StreamTrait, InputCallbackInfo};
use ndarray::{arr1, concatenate, s, Array1, Array2, Axis};

use crate::{
    audio::new_audio_stream,
    config::Config,
    dsp::{self, Dsp},
    led::ESP8266Conn,
};

pub enum Changed {
    Buffer,
    Config,
    SelectedPreset,
}

pub struct SharedRenderState {
    display_buffer: Array2<f64>,
    display_buffer_quantized: Array2<u8>,
    config: Config,
    selected_preset: dsp::Preset,
    changes: Vec<Changed>,
}

impl SharedRenderState {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            display_buffer: Array2::<f64>::zeros((config.n_points as usize, 3)),
            display_buffer_quantized: Array2::<u8>::zeros((config.n_points as usize, 3)),
            selected_preset: dsp::Preset::Scroll,
            changes: Vec::new(),
        }
    }
}

pub struct Renderer {
    state: Arc<Mutex<SharedRenderState>>,
    rolling_history: Array1<f64>,
    last_render: Instant,
    frame_duration: Duration,
    config: Config,
    conn: ESP8266Conn,
    dsp: Dsp,
}

impl Renderer {
    pub fn new(state: Arc<Mutex<SharedRenderState>>) -> Self {
        let config = state.lock().unwrap().config.clone();
        let frame_duration = Duration::from_secs_f64(1. / config.fps as f64);

        Self {
            state,
            rolling_history: Array1::<f64>::zeros(config.n_fft_bins as usize),
            frame_duration,
            last_render: Instant::now() - frame_duration, // start rendering on our first sample
            config: config.clone(),
            conn: ESP8266Conn::new(&config).unwrap(),
            dsp: Dsp::new(config),
        }
    }

    pub fn main_loop(mut self) {
        let stream = new_audio_stream(
            self.config,
            move |audio_data: &[f32], info: &InputCallbackInfo| {
                let mut state = self.state.lock().unwrap();

                // move in new audio samples to buffer (back is newest)
                let new_data = arr1(audio_data).mapv(f64::from);
                self.rolling_history = concatenate![
                    Axis(0),
                    self.rolling_history.slice(s![new_data.shape()[0]..]),
                    new_data
                ];

                // re-render when we encounter a frame boundary
                if self.last_render.elapsed() > self.frame_duration {
                    self.last_render = Instant::now();

                    // transform the audio to the frequency space and then to the mel spectrum
                    let audio_data_rfft = self.dsp.exec_rfft(&self.rolling_history);
                    let mut audio_data_mel = self.dsp.get_mel_repr(&audio_data_rfft);
                    self.dsp.gain_and_smooth(&mut audio_data_mel);

                    let new_display_buffer: Array2<f64> = self
                        .dsp
                        .apply_transform(state.selected_preset.clone(), &mut state.display_buffer);

                    let mut new_display_buffer_quantized: Array2<u8> =
                        new_display_buffer.map(|v| {
                            if *v < 0.0 {
                                0
                            } else if *v > 255.0 {
                                255
                            } else {
                                *v as u8
                            }
                        });

                    self.conn
                        .update(
                            &mut new_display_buffer_quantized,
                            &state.display_buffer_quantized,
                        )
                        .expect("error updating connection");

                    state.display_buffer_quantized = new_display_buffer_quantized;
                    state.display_buffer = new_display_buffer;
                }
            },
        );
        stream.play().unwrap();
    }
}
