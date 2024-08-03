use dirs::home_dir;
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use crate::args::Args;

pub static DEFAULT_CONFIG_PATH: &str = ".config/audio-reactive-led-strip/config.toml";

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub device_ip: String,
    pub device_port: u32,
    pub use_gui: bool,
    pub software_gamma_correction: bool,
    pub n_points: u8,
    pub mic_rate: u32,
    pub fps: u32,
    pub min_frequency: u32,
    pub max_frequency: u32,
    pub n_fft_bins: u32,
    pub n_rolling_history: u32,
    pub min_volume_threshold: f64,
    pub left_slider_start: u32,
    pub right_slider_start: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            device_ip: String::from("192.168.0.150"),
            device_port: 7777,
            software_gamma_correction: true,
            use_gui: false,
            n_points: 255,
            mic_rate: 44100,
            fps: 60,
            min_frequency: 200,
            max_frequency: 12000,
            n_fft_bins: 24,
            n_rolling_history: 2,
            min_volume_threshold: 1e-7,
            left_slider_start: 200,
            right_slider_start: 20000,
        }
    }
}

impl Config {
    pub fn merge_with_args(&mut self, args: Args) {
        self.use_gui |= args.use_gui;
        if let Some(device_ip) = args.device_ip {
            self.device_ip = device_ip
        }
        if let Some(device_port) = args.device_port {
            self.device_port = device_port
        }
    }
}

pub fn load_config(path_str: &String, use_home_dir: bool) -> Config {
    let mut path = Path::new(path_str);
    let mut path_buf: PathBuf;
    if use_home_dir {
        path_buf = home_dir().unwrap();
        path_buf.push(path);
        path = path_buf.as_path();
    }
    let mut source = String::new();

    let read_file = match File::open(path) {
        Err(_) => {
            println!(
                "Could not open path {}, loading default config",
                path.display()
            );
            return Config::default();
        }
        Ok(mut file) => file.read_to_string(&mut source),
    };

    if let Err(e) = read_file {
        println!(
            "Could read config due to an error: {:?}. Loading default config instead",
            e
        );
        return Config::default();
    }

    let parsed_toml = toml::from_str::<Config>(source.as_str());
    match parsed_toml {
        Ok(config) => config,
        Err(err) => {
            println!(
                "Error parsing config toml: {:?}. Loading default config instead",
                err
            );
            Config::default()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_config_path_dne() {
        let default_conf = load_config(&String::from("path_does_not_exist"), false);
        assert_eq!(default_conf, Config::default());
    }

    #[test]
    fn test_load_example_config() {
        load_config(&String::from("test/config.toml"), false);
    }

    #[test]
    fn test_load_config_error() {
        load_config(&String::from("test/config_error.toml"), false);
    }
}
