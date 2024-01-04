use std::{sync::Arc, usize};

use ndarray::{s, Array, Array1, Array2};
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
    use ndarray::arr1;
    use ndarray_npy::NpzReader;

    use super::*;

    #[test]
    fn test_mel_hz_conversion() {
        assert_abs_diff_eq!(mel_to_hertz(100.0), 64.95112114434983);
        assert_abs_diff_eq!(hertz_to_mel(100.0), 150.48910240709708);
    }

    #[test]
    fn test_exp_filter() {
        let expected = ndarray::arr1(&[
            0.19133972, 0.0466747, 0.78939848, 0.12620031, 0.84045104, 0.12606363, 0.09315127,
            0.27223475, 0.32086958, 0.54406098, 0.27306298, 0.07422397, 0.3567698, 0.63903146,
            0.54285225, 0.8732071, 0.05015262, 0.17689518, 0.21245038, 0.39821787, 0.17053282,
            0.96182047, 0.19629877, 0.16890264, 0.42743507, 0.05561293, 0.64582918, 0.28475749,
            0.38559554, 0.30715199, 0.06509077, 0.56056894, 0.74032583, 0.58646942, 0.14819371,
            0.71098846, 0.10836583, 0.18009493, 1.03677603, 0.0939396, 0.19987542, 0.71693586,
            0.75004527, 0.62607192, 0.81776577, 0.55823947, 0.60164642, 0.32214651, 0.25142108,
            0.01572098, 0.98233563, 0.08420938, 0.20684954, 1.47828133, 0.56246047, 0.66047807,
            0.439108, 0.21602114, 0.56722197, 0.43276743, 0.38860071, 0.50480968, 0.28100791,
            0.09338005, 0.3620024, 1.01813112, 0.88354865, 0.17115398, 0.38873698, 0.13448836,
            0.01481685, 1.08462796, 0.42399389, 0.15059725, 0.21701932, 0.22097095, 0.20424413,
            0.06263462, 0.67498377, 1.14723017, 0.14540819, 0.15239976, 0.31702893, 0.39450086,
            1.24415747, 0.14300254, 0.12849241, 0.25839275, 0.10482977, 0.17225263, 0.13979815,
            0.37850805, 0.67066182, 0.13482642, 0.23988855, 0.09029223, 0.58151291, 0.36260366,
            0.15923866, 0.9093225, 0.29593426, 1.26483096, 0.03955397, 0.28615806, 0.25365253,
            0.25355763, 0.69417152, 0.61566914, 0.29388144, 0.23124628, 0.18343216, 0.13974017,
            0.77269906, 0.4325813, 0.14269003, 0.14469567, 0.45572424, 1.16938906, 1.21256352,
            0.60719386, 0.36757455, 0.39430982, 0.61748262, 0.29613119, 1.12228896, 0.28436277,
            0.00991962,
        ]);

        let update = ndarray::arr1(&[
            0.372679438,
            0.0833493948,
            1.56879697,
            0.242400624,
            1.67090209,
            0.242127251,
            0.176302539,
            0.534469501,
            0.631739156,
            1.07812196,
            0.536125964,
            0.138447941,
            0.703539597,
            1.26806292,
            1.07570449,
            1.7364142,
            0.0903052314,
            0.343790353,
            0.414900752,
            0.786435734,
            0.331065632,
            1.91364094,
            0.382597547,
            0.327805286,
            0.844870141,
            0.101225865,
            1.28165836,
            0.559514986,
            0.761191086,
            0.604303975,
            0.120181532,
            1.11113788,
            1.47065166,
            1.16293883,
            0.286387423,
            1.41197693,
            0.206731657,
            0.350189855,
            2.06355207,
            0.177879204,
            0.389750849,
            1.42387171,
            1.49009053,
            1.24214383,
            1.62553154,
            1.10647894,
            1.19329284,
            0.63429302,
            0.492842163,
            0.0214419542,
            1.95467126,
            0.158418753,
            0.403699089,
            2.94656267,
            1.11492094,
            1.31095615,
            0.868216009,
            0.422042283,
            1.12444395,
            0.855534859,
            0.767201418,
            0.999619357,
            0.552015829,
            0.176760109,
            0.714004797,
            2.02626224,
            1.7570973,
            0.332307967,
            0.767473965,
            0.258976729,
            0.0196337098,
            2.15925591,
            0.837987784,
            0.291194494,
            0.424038647,
            0.431941904,
            0.398488252,
            0.115269235,
            1.33996755,
            2.28446035,
            0.280816374,
            0.294799529,
            0.624057863,
            0.779001725,
            2.47831494,
            0.276005083,
            0.246984811,
            0.506785503,
            0.199659542,
            0.334505257,
            0.269596305,
            0.747016091,
            1.33132365,
            0.259652834,
            0.469777095,
            0.170584454,
            1.15302582,
            0.715207317,
            0.308477316,
            1.808645,
            0.58186853,
            2.51966192,
            0.0691079306,
            0.562316112,
            0.497305052,
            0.497115255,
            1.37834304,
            1.22133827,
            0.577762874,
            0.45249256,
            0.356864318,
            0.269480333,
            1.53539812,
            0.855162608,
            0.27538005,
            0.279391331,
            0.901448475,
            2.32877813,
            2.41512705,
            1.20438772,
            0.725149099,
            0.778619649,
            1.22496524,
            0.582262387,
            2.23457791,
            0.558725532,
            0.00196216942,
        ]);

        let current = ndarray::Array::ones(127) * 0.01;
        let output = exp_filter_array(&current, &update, 0.1, 0.5);
        assert!(output.abs_diff_eq(&expected, 0.1));
    }

    #[test]
    fn test_rfft() {
        let input = arr1(&[
            -0.50168074,
            -1.36283763,
            2.08719592,
            0.56672292,
            -0.9343916,
            -0.27035682,
            -0.55382359,
            0.62773806,
            0.56436091,
            -0.57617775,
            2.6793696,
            0.53109647,
            1.1987642,
            -0.67969233,
            -0.22888863,
            -0.69824528,
        ]);
        let expected = arr1(&[
            2.44915373, 3.98812057, 5.93305473, 2.25434631, 5.35818967, 1.22848397, 5.58169067,
            3.50134391, 6.17265841,
        ]);

        let input2 = arr1(&[
            0., 0.09983342, 0.19866933, 0.29552021, 0.38941834, 0.47942554, 0.56464247, 0.64421769,
            0.71735609, 0.78332691, 0.84147098, 0.89120736, 0.93203909, 0.96355819, 0.98544973,
            0.99749499,
        ]);

        let expected2 = arr1(&[
            9.78363033, 2.95376068, 1.40243711, 0.95359203, 0.74589837, 0.63307384, 0.5691891,
            0.53591029, 0.52553825,
        ]);

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
