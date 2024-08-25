use std::sync::{Arc, Mutex};

use cpal::InputCallbackInfo;
use ndarray::{arr1, Array2};

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

pub struct Renderer {
    state: Arc<Mutex<SharedRenderState>>,
    config: Config,
    conn: ESP8266Conn,
    dsp: Dsp,
}

impl Renderer {
    fn new(state: Arc<Mutex<SharedRenderState>>) -> Self {
        let config = state.lock().unwrap().config.clone();
        Self {
            state,
            config: config.clone(),
            conn: ESP8266Conn::new(&config).unwrap(),
            dsp: Dsp::new(config, 0.2, 0.2),
        }
    }

    fn main_loop(mut self) {
        new_audio_stream(
            self.config,
            move |audio_data: &[f64], info: &InputCallbackInfo| {
                let mut state = self.state.lock().unwrap();
                let audio_data_rfft = arr1(audio_data);
                self.dsp.exec_rfft(&audio_data_rfft);
                let audio_data_mel = self.dsp.get_mel_repr(&audio_data_rfft);

                let new_display_buffer: Array2<f64> = self
                    .dsp
                    .apply_transform(state.selected_preset.clone(), &mut state.display_buffer);

                let mut new_display_buffer_quantized: Array2<u8> = new_display_buffer.map(|v| {
                    if *v < 0.0 {
                        0
                    } else if *v > 255.0 {
                        255
                    } else {
                        *v as u8
                    }
                });

                self.conn.update(
                    &mut new_display_buffer_quantized,
                    &state.display_buffer_quantized,
                );

                state.display_buffer_quantized = new_display_buffer_quantized;
                state.display_buffer = new_display_buffer;
            },
        );
    }
}
