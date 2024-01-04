use std::{fs::File, io::Read};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    device_ip: String,
    device_port: u32,
    software_gamma_correction: bool,
    use_gui: bool,
    n_pixels: u8,
    mic_rate: u32,
    fps: u32,
    min_frequency: u32,
    max_frequency: u32,
    n_fft_bins: u32,
    n_rolling_history: u32,
    min_volume_threshold: f64,
}

pub fn default_config() -> Config {
    Config {
        device_ip: String::from("192.168.0.150"),
        device_port: 7777,
        software_gamma_correction: true,
        use_gui: true,
        n_pixels: 255,
        mic_rate: 44100,
        fps: 60,
        min_frequency: 200,
        max_frequency: 12000,
        n_fft_bins: 24,
        n_rolling_history: 2,
        min_volume_threshold: 1e-7,
    }
}
pub fn load_config(path: &String) -> Result<Config, toml::de::Error> {
    let mut source = String::new();

    let read_file = match File::open(path) {
        Err(_) => {
            println!("Could not open path {}, loading default config", path);
            return Ok(default_config());
        }
        Ok(mut file) => file.read_to_string(&mut source),
    };

    if let Err(e) = read_file {
        println!(
            "Could read config due to an error: {:?}. Loading default config instead",
            e
        );
        Ok(default_config())
    } else {
        toml::from_str::<Config>(source.as_str())
    }
}
