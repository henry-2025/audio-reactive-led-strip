use cpal::InputCallbackInfo;
use ndarray::arr1;

use crate::{
    audio::new_audio_stream,
    config::Config,
    dsp::{create_mel_bank, exec_rfft, new_rfft},
    led::ESP8266Conn,
};

fn main_loop(config: Config) {
    let led_conn = ESP8266Conn::new(&config).unwrap();
    let mel_bank = create_mel_bank(
        config.mic_rate,
        config.n_rolling_history,
        config.fps,
        config.n_fft_bins,
        config.min_frequency,
        config.max_frequency,
    );
    let callback = |audio_data: &[f64], info: &InputCallbackInfo| {
        let rfft = new_rfft(audio_data.len());
        let audio_data_rfft = arr1(audio_data);
        exec_rfft(&audio_data_rfft, &rfft);
        let audio_data_mel = mel_bank.x * audio_data_rfft;
        let update_pixels = mel_bank.led_conn.update();
    };
    let audio_stream = new_audio_stream(&config, callback);
}
