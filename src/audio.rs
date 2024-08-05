use cpal::{
    default_host,
    traits::{DeviceTrait, HostTrait},
    InputCallbackInfo, SampleFormat, SampleRate, Stream, StreamError, SupportedStreamConfig,
};

use crate::config::Config;

pub fn new_audio_stream<D>(config: &Config, update_callback: D) -> Stream
where
    D: FnMut(&[f64], &InputCallbackInfo) + Send + 'static,
{
    let device = default_host()
        .default_input_device()
        .expect("No default input device could be bound");
    let configs: Vec<SupportedStreamConfig> = device
        .supported_input_configs()
        .unwrap()
        .filter_map(|x| {
            //TODO: return an error if the sample rate is not supported. Default behavior is that
            //this panics
            //TODO: this is also a pretty ugly nested let
            if x.max_sample_rate() < SampleRate(config.mic_rate)
                || x.min_sample_rate() > SampleRate(config.mic_rate)
            {
                None
            } else {
                let sample_rates = x.with_sample_rate(SampleRate(config.mic_rate));
                //TODO: for now, mac only supports floating-point sampling formats. In the future,
                //will want to compile to support i16 and u16 formats as well. Will be a good
                //case for pattern matching
                if sample_rates.sample_format() == SampleFormat::F64 && sample_rates.channels() == 1
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
            "Could not create the intended audio input config: 1 channel, {}Hz, i16 format",
            config.mic_rate
        );
    }
    device
        .build_input_stream(
            &configs[0].config(),
            update_callback,
            |_: StreamError| {
                println!("error");
            },
            None,
        )
        .expect("Could not build audio stream")
}

#[cfg(test)]
mod test {
    use cpal::InputCallbackInfo;

    use crate::config::Config;

    use super::new_audio_stream;

    #[test]
    fn test_create_audio_stream() {
        fn test_callback(_: &[f64], _: &InputCallbackInfo) {}
        new_audio_stream(&Config::default(), test_callback);
    }
}
