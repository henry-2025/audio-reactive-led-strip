use std::{fs::File, io::Read};

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub device_ip: String,
    pub device_port: u32,
    pub software_gamma_correction: bool,
    pub use_gui: bool,
    pub n_pixels: u8,
    pub mic_rate: u32,
    pub fps: u32,
    pub min_frequency: u32,
    pub max_frequency: u32,
    pub n_fft_bins: u32,
    pub n_rolling_history: u32,
    pub min_volume_threshold: f64,
}

impl Default for Config {
    fn default() -> Self {
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
}

pub fn load_config(path: &String) -> Result<Config, toml::de::Error> {
    let mut source = String::new();

    let read_file = match File::open(path) {
        Err(_) => {
            println!("Could not open path {}, loading default config", path);
            return Ok(Config::default());
        }
        Ok(mut file) => file.read_to_string(&mut source),
    };

    if let Err(e) = read_file {
        println!(
            "Could read config due to an error: {:?}. Loading default config instead",
            e
        );
        Ok(Config::default())
    } else {
        toml::from_str::<Config>(source.as_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_config_path_dne() {
        let default_conf = load_config(&String::from("path_does_not_exist")).unwrap();
        assert_eq!(default_conf, Config::default());
    }

    #[test]
    fn test_load_example_config() {
        load_config(&String::from("test/config.toml")).unwrap();
    }

    #[test]
    fn test_load_config_error() {
        load_config(&String::from("test/config_error.toml"))
            .expect_err("Expected a parsing error from this");
    }
}
