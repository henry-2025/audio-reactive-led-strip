use std::{
    sync, thread,
    time::{Duration, Instant},
};

use crate::gui::waveform::pipeline::Vertex;
use cpal::{traits::StreamTrait, InputCallbackInfo};
use iced::futures::channel::mpsc::{Receiver, Sender};
use ndarray::{arr1, concatenate, s, Array1, Array2, Axis};

use crate::{
    audio::new_audio_stream,
    config::Config,
    dsp::{self, Dsp},
    gui::GuiMessage,
    led::ESP8266Conn,
};

pub struct Renderer {
    display_values: Array2<f64>,
    send_buffer: Array2<u8>,
    selected_preset: dsp::Preset,
    rolling_history: Array1<f64>,
    last_render: Instant,
    update_tx: Option<Sender<GuiMessage>>,
    frame_duration: Duration,
    config: Config,
    conn: ESP8266Conn,
    dsp: Dsp,
    ready: bool,
}

impl Renderer {
    pub fn new(config: Config, update_tx: Option<Sender<GuiMessage>>) -> Self {
        let frame_duration = Duration::from_secs_f64(1. / config.fps as f64);

        Self {
            display_values: Array2::<f64>::zeros((config.n_points as usize, 3)),
            send_buffer: Array2::<u8>::zeros((config.n_points as usize, 3)),
            selected_preset: dsp::Preset::Scroll,
            rolling_history: Array1::<f64>::zeros(config.n_fft_bins as usize),
            frame_duration,
            update_tx,
            last_render: Instant::now() - frame_duration, // start rendering on our first sample
            config: config.clone(),
            conn: ESP8266Conn::new(&config).unwrap(),
            dsp: Dsp::new(config),
            ready: false,
        }
    }

    fn apply_updates(&mut self, u: GuiMessage) {}

    pub fn main_loop(
        mut self,
        stop: sync::mpsc::Receiver<()>,
        in_channel: Receiver<GuiMessage>,
        out_channel: Sender<Array2<u8>>,
    ) {
        let stream = new_audio_stream(
            self.config.clone(),
            move |audio_data: &[f32], _: &InputCallbackInfo| {
                self.update(audio_data);
            },
        );
        stream.play().expect("error playing audio stream");
        stop.recv().expect("error accepting thread stop signal");
    }

    fn update(&mut self, audio_data: &[f32]) {
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

            self.dsp
                .apply_transform_inplace(self.selected_preset.clone(), &mut self.display_values);

            let mut new_send_buffer: Array2<u8> = self.display_values.map(|v| {
                if *v < 0.0 {
                    0
                } else if *v > 255.0 {
                    255
                } else {
                    *v as u8
                }
            });

            self.conn
                .update(&mut new_send_buffer, &self.send_buffer)
                .expect("error updating connection");

            self.send_buffer = new_send_buffer;
        }
    }

    pub fn main_loop_external_updates(mut self) {
        let (stop_tx, stop_rx) = sync::mpsc::channel::<()>();
        self.update_tx
            .as_mut()
            .expect("update tx should exist in main loop setup")
            .try_send(GuiMessage::StopTx(stop_tx))
            .expect("update tx should be ready to accept messages");

        let stream = new_audio_stream(
            self.config.clone(),
            move |audio_data: &[f32], _: &InputCallbackInfo| {
                if !self.ready {
                    // do thread communication init here
                    self.ready = true;
                }
                self.update(audio_data);

                self.update_tx
                    .as_mut()
                    .expect("update tx should exist in render thread")
                    .try_send(GuiMessage::PointsUpdated(send_buffer_to_vertex(
                        &self.send_buffer,
                    )))
                    .expect("send points update should succeed if channel is open");
            },
        );
        stream.play().expect("audio stream should be ready to play");
        stop_rx
            .recv()
            .expect("stop receiver exists and should not have been closed");
    }
}

fn send_buffer_to_vertex(send_buffer: &Array2<u8>) -> Vec<Vertex> {
    send_buffer
        .axis_iter(Axis(0))
        .map(|col| Vertex([col[0] as i32, col[1] as i32, col[2] as i32]))
        .collect::<Vec<Vertex>>()
}
