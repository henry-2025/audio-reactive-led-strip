use std::{sync::Arc, usize};

use ndarray::{Array1, Array2};
use rustfft::{
    num_complex::{Complex64, ComplexFloat},
    Fft, FftPlanner,
};

fn hertz_to_mel(hertz: f64) -> f64 {
    2595.0 * (1.0 + (hertz / 700.0)).log10()
}

fn mel_to_hertz(mel: f64) -> f64 {
    700.0 * (10.0.powf(mel / 2595.0)) - 700.0
}

/**
 * Single exponential filter value. Returns the new value to the stack
 */
fn exp_filter_single(current_val: f64, new_val: f64, alpha_decay: f64, alpha_rise: f64) -> f64 {
    let alpha = if new_val > current_val {
        alpha_rise
    } else {
        alpha_decay
    };
    alpha * new_val + (1.0 - alpha) * current_val
}

fn exp_filter_array(
    current: &ndarray::Array1<f64>,
    new: &ndarray::Array1<f64>,
    alpha_decay: f64,
    alpha_rise: f64,
) -> ndarray::Array1<f64> {
    assert_eq!(current.len(), new.len());
    current
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let alpha = if new[i] - c > 0.0 {
                alpha_rise
            } else {
                alpha_decay
            };
            alpha * new[i] + (1.0 - alpha) * c
        })
        .collect()
}

pub fn new_rfft(fft_size: usize) -> Arc<dyn Fft<f64>> {
    FftPlanner::new().plan_fft_forward(fft_size)
}

pub fn exec_rfft(buffer: &Array1<f64>, fft: &Arc<dyn Fft<f64>>) -> Array1<f64> {
    let mut complex_buffer: Array1<Complex64> =
        buffer.iter().map(|x| Complex64::new(*x, 0.0)).collect();

    fft.process(complex_buffer.as_slice_mut().unwrap());

    complex_buffer
        .iter()
        .enumerate()
        .filter_map(|(i, x)| {
            if i <= buffer.len() / 2 {
                Some(x.abs())
            } else {
                None
            }
        })
        .collect()
}

/**
 * A transformation matrix for mel spectrum
 * mel\_y: the transformation matrix
 * mel\_x: the center frequencies of the mel bands
 */
pub struct MelBank {
    x: Array1<f64>,
    y: Array2<f64>,
}

/**
* Generate a MelBank according to parameters
* mic\_rate: the sampling rate of the microphone
* n\_rolling\_history: the number of samples in the rolling history
*/
pub fn create_mel_bank(
    mic_rate: u32,
    n_rolling_history: u32,
    fps: u32,
    n_fft_bins: u32,
    min_freq_hz: u32,
    max_freq_hz: u32,
) -> MelBank {
    let num_fft_bands = (mic_rate * n_rolling_history) as f64 / (2.0 * fps as f64);
    let num_mel_bands = n_fft_bins as usize; // bands and bins are different quantities

    // generate centerfrequencies and band eges for a mel filter bank
    let max_freq_mel = hertz_to_mel(max_freq_hz as f64);
    let min_freq_mel = hertz_to_mel(min_freq_hz as f64);
    let delta_mel = (max_freq_mel - min_freq_mel).abs() / (num_mel_bands as f64 + 1.0);
    let frequencies_mel =
        Array1::from_iter((0..num_mel_bands + 2).map(|i| i as f64)) * delta_mel + min_freq_mel;
    let frequencies_hz = frequencies_mel.map(|mel| mel_to_hertz(*mel));

    // build the mel input scale and transformation matrix
    let mel_x = ndarray::Array1::linspace(0., mic_rate as f64 / 2.0, num_fft_bands as usize);
    let mel_y = Array2::from_shape_fn((num_mel_bands, num_fft_bands as usize), |(i, j)| {
        let (lower, center, upper) = (
            frequencies_hz[i],
            frequencies_hz[i + 1],
            frequencies_hz[i + 2],
        );
        if (mel_x[j] >= lower) == (mel_x[j] <= center) {
            (mel_x[j] - lower) / (center - lower)
        } else if (mel_x[j] >= center) == (mel_x[j] <= upper) {
            (upper - mel_x[j]) / (upper - center)
        } else {
            0.0
        }
    });

    MelBank { x: mel_x, y: mel_y }
}

#[cfg(test)]
mod test {
    use std::fs::File;

    use approx::assert_abs_diff_eq;
    use ndarray_npy::NpzReader;

    use super::*;

    #[test]
    fn test_mel_hz_conversion() {
        assert_abs_diff_eq!(mel_to_hertz(100.0), 64.95112114434983);
        assert_abs_diff_eq!(hertz_to_mel(100.0), 150.48910240709708);
    }

    #[test]
    fn test_exp_filter() {
        let mut npz_reader =
            NpzReader::new(File::open("./test/exp_filter_test.npz").unwrap()).unwrap();

        let expected: Array1<f64> = npz_reader.by_name("expected.npy").unwrap();
        let update: Array1<f64> = npz_reader.by_name("update.npy").unwrap();

        let current = ndarray::Array::ones(127) * 0.01;
        let output = exp_filter_array(&current, &update, 0.1, 0.5);
        assert!(output.abs_diff_eq(&expected, 0.1));
    }

    #[test]
    fn test_rfft() {
        let mut npz_reader = NpzReader::new(File::open("./test/rfft_test.npz").unwrap()).unwrap();

        let input: Array1<f64> = npz_reader.by_name("input.npy").unwrap();
        let expected: Array1<f64> = npz_reader.by_name("expected.npy").unwrap();
        let input2: Array1<f64> = npz_reader.by_name("input2.npy").unwrap();
        let expected2: Array1<f64> = npz_reader.by_name("expected2.npy").unwrap();

        let fft = new_rfft(16);
        let output = exec_rfft(&input, &fft);

        let epsilon = 1e-3;
        assert_eq!(output.len(), expected.len());
        assert!(output.abs_diff_eq(&expected, epsilon));
        let output2 = exec_rfft(&input2, &fft);
        assert!(output2.abs_diff_eq(&expected2, epsilon));
    }

    #[test]
    fn test_create_mel_bank() {
        let output = create_mel_bank(44100, 2, 60, 24, 200, 12000);
        let mut npz_reader = NpzReader::new(File::open("./test/mel_test.npz").unwrap()).unwrap();

        let expected = MelBank {
            x: npz_reader.by_name("mel_x.npy").unwrap(),
            y: npz_reader.by_name("mel_y.npy").unwrap(),
        };

        let epsilon = 1e-3;
        assert!(output.x.abs_diff_eq(&expected.x, epsilon));
        assert!(output.y.abs_diff_eq(&expected.y, epsilon));
    }
}
