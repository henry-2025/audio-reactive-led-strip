use std::sync::Arc;

use ndarray::{s, Array, Array1, Array2, Axis, Dimension, Ix1, Ix2, NewAxis};
use rustfft::{
    num_complex::{Complex64, ComplexFloat},
    Fft, FftPlanner,
};

use crate::config::Config;

/*
===To add new transforms===

1. create a transform function with the signature
fn(display_buffer: &mut Array2<f64>,
y: &mut Array1<f64>,
gain: &mut Array1<f64>,
alpha_decay: f64,
alpha_rise: f64)

2. create a preset enum

3. assign the preset enum to the function in the apply_transform match
*/
pub struct Dsp {
    gain: ExpFilterArr<Ix1>,
    p_filt: ExpFilterArr<Ix2>,
    common_mode: ExpFilterArr<Ix1>,
    r_filt: ExpFilterArr<Ix1>,
    g_filt: ExpFilterArr<Ix1>,
    b_filt: ExpFilterArr<Ix1>,
    prev_spectrum: Array1<f64>,
    gaussian_kernel1: Array1<f64>,
    gaussian_kernel2: Array1<f64>,
    mel_bank: MelBank,
    fft: Arc<dyn Fft<f64>>,
    config: Config,
    alpha_decay: f64,
    alpha_rise: f64,
}

#[derive(Clone)]
pub enum Preset {
    Scroll,
    Power,
    Spectrum,
}

impl Dsp {
    pub fn new(config: Config, alpha_rise: f64, alpha_decay: f64) -> Self {
        Self {
            gain: ExpFilterArr::<Ix1>::new(
                (config.n_points / 2) as u32,
                0.01,
                alpha_rise,
                alpha_decay,
            ),
            p_filt: ExpFilterArr::<Ix2>::new((config.n_points / 2) as u32, 1., 0.99, 0.1),
            common_mode: ExpFilterArr::<Ix1>::new((config.n_points / 2) as u32, 0.01, 0.99, 0.01),
            r_filt: ExpFilterArr::<Ix1>::new((config.n_points / 2) as u32, 0.01, 0.2, 0.99),
            g_filt: ExpFilterArr::<Ix1>::new((config.n_points / 2) as u32, 0.01, 0.05, 0.3),
            b_filt: ExpFilterArr::<Ix1>::new((config.n_points / 2) as u32, 0.01, 0.1, 0.5),
            prev_spectrum: Array1::zeros(config.n_points as usize),
            gaussian_kernel1: gaussian_kernel(0.2, 0, 1), // TODO: determine whether radius 1 is what we want
            gaussian_kernel2: gaussian_kernel(0.4, 0, 1), // TODO: determine whether radius 1 is what we want
            mel_bank: create_mel_bank(
                config.mic_rate,
                config.n_rolling_history,
                config.fps,
                config.n_fft_bins,
                config.min_freq_hz,
                config.max_freq_hz,
            ),
            fft: new_rfft(config.n_fft_bins),
            config,
            alpha_decay,
            alpha_rise,
        }
    }
    pub fn apply_transform(
        &mut self,
        preset: Preset,
        display_buffer: &mut Array2<f64>,
    ) -> Array2<f64> {
        match preset {
            Preset::Scroll => self.visualize_scroll(display_buffer),
            Preset::Power => self.visualize_power(display_buffer),
            Preset::Spectrum => self.visualize_spectrum(display_buffer),
        }
    }

    fn visualize_scroll(&mut self, display_buffer: &mut Array2<f64>) -> Array2<f64> {
        /*
        global p
            y = y**2.0
            gain.update(y)
            y /= gain.value
            y *= 255.0
            r = int(np.max(y[:len(y) // 3]))
            g = int(np.max(y[len(y) // 3: 2 * len(y) // 3]))
            b = int(np.max(y[2 * len(y) // 3:]))
            # Scrolling effect window
            p[:, 1:] = p[:, :-1]
            p *= 0.98
            p = gaussian_filter1d(p, sigma=0.2)
            # Create new color originating at the center
            p[0, 0] = r
            p[1, 0] = g
            p[2, 0] = b
            # Update the LED strip
            return np.concatenate((p[:, ::-1], p), axis=1)
        */
        let mut y = self.mel_bank.x.clone();
        // y = y**2.0
        y.map_inplace(|x| *x = x.powi(2));
        // update gain
        self.gain.update(&y);
        // y /= gain.value
        // y *= 255
        y.zip_mut_with(&self.gain.current, |y, g| *y = 255.0 * (*y) / g);

        // scrolling effect
        // p[:, 1:] = p[:, :-1]
        // p *= 0.98
        for i in 1..y.shape()[0] - 1 {
            for j in 0..3 {
                display_buffer[[i, j]] = display_buffer[[i - 1, j]] * 0.98;
            }
        }
        // apply gaussian filter
        let mut filter_display_buffer = correlate_1d(display_buffer, &self.gaussian_kernel1);

        // create one new color originating at the center
        for i in 0..3 {
            let s = y.slice(s![i * y.shape()[0] / 3..(i + 1) * y.shape()[0] / 3]);
            let mut max: f64 = 0.0;
            s.map(|x| {
                max = f64::max(max, *x);
            });

            filter_display_buffer[[0, i]] = max;
        }

        // scroll display
        ndarray::concatenate![
            Axis(0),
            filter_display_buffer,
            filter_display_buffer.slice(s![..,..;-1])
        ]
    }
    fn visualize_power(&mut self, display_buffer: &mut Array2<f64>) -> Array2<f64> {
        let mut y = self.mel_bank.x.clone();
        self.gain.update(&y);

        // y /= gain.value
        // y *= float(config.n_pixels // 2) - 1)
        y.zip_mut_with(&self.gain.current, |y, g| {
            *y = ((self.config.n_points / 2) - 1) as f64 * (*y)
        });

        // map color channels according to energy in different frequency bands
        let scale = 0.9;
        for i in 0..3 {
            let s = y
                .slice(s![i * y.shape()[0] / 3..(i + 1) * y.shape()[0] / 3])
                .map(|x| x.powf(scale));
            let mean = s.mean().unwrap() as usize;
            display_buffer.slice_mut(s![i, ..mean]).fill(255.0);
            display_buffer.slice_mut(s![i, mean..]).fill(0.0);
        }

        self.p_filt.update(display_buffer);
        display_buffer.map_inplace(|x| {
            *x = x.round();
        });
        display_buffer.assign(&correlate_1d(display_buffer, &self.gaussian_kernel2));

        return ndarray::concatenate![Axis(0), display_buffer.slice(s![..,..;-1]), *display_buffer];
    }
    fn visualize_spectrum(&mut self, _: &mut Array2<f64>) -> Array2<f64> {
        // TODO: need to do interpolation up here?
        let mut y = self.mel_bank.x.clone();
        self.common_mode.update(&y);
        //diff = y - self.prev_spectrum
        let diff = &y - &self.prev_spectrum;
        self.prev_spectrum.assign(&y);

        // color channel mappings
        self.r_filt.update(&(&y - &self.common_mode.current));
        let r = &self.r_filt.current;
        let g = diff.map(|x| x.abs());
        self.b_filt.update(&y);
        let b = &self.b_filt.current;

        // Mirror the color channels for symmetric output
        let r = ndarray::concatenate![Axis(0), r.slice(s![..;-1]).to_owned(), r.to_owned()];
        let g = ndarray::concatenate![Axis(0), g.slice(s![..;-1]), g];
        let b = ndarray::concatenate![Axis(0), b.slice(s![..;-1]).to_owned(), b.to_owned()];

        ndarray::stack![Axis(0), r, g, b]
    }

    pub fn exec_rfft(&self, buffer: &Array1<f64>) -> Array1<f64> {
        let mut complex_buffer: Array1<Complex64> =
            buffer.iter().map(|x| Complex64::new(*x, 0.0)).collect();

        self.fft.process(complex_buffer.as_slice_mut().unwrap());

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

    pub fn get_mel_repr(&self, audio: &Array1<f64>) -> Array1<f64> {
        self.mel_bank.y.dot(audio)
    }

    pub fn gaussian_filter1d(&self, input: &Array2<f64>) -> Array2<f64> {
        println!("{:?}", &self.gaussian_kernel1.slice(s![..;-1]).to_owned());
        correlate_1d(input, &self.gaussian_kernel1.slice(s![..;-1]).to_owned())
    }
}

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

struct ExpFilterArr<T>
where
    T: Dimension,
{
    current: Array<f64, T>,
    alpha_rise: f64,
    alpha_decay: f64,
}

impl ExpFilterArr<Ix2> {
    pub fn new(size: u32, init: f64, alpha_rise: f64, alpha_decay: f64) -> Self {
        Self {
            current: Array::<f64, Ix2>::ones((3, size as usize)) * init,
            alpha_rise,
            alpha_decay,
        }
    }
    pub fn update(&mut self, new: &Array2<f64>) {
        assert_eq!(self.current.len(), new.len());
        self.current.indexed_iter_mut().for_each(|(i, c)| {
            let alpha = if new[i] - *c > 0.0 {
                self.alpha_rise
            } else {
                self.alpha_decay
            };
            *c = alpha * new[i] + (1.0 - alpha) * (*c);
        })
    }
}

impl ExpFilterArr<Ix1> {
    pub fn new(size: u32, init: f64, alpha_rise: f64, alpha_decay: f64) -> Self {
        Self {
            current: Array::<f64, Ix1>::ones(size as usize) * init,
            alpha_rise,
            alpha_decay,
        }
    }
    pub fn update(&mut self, new: &Array1<f64>) {
        assert_eq!(self.current.len(), new.len());
        self.current.indexed_iter_mut().for_each(|(i, c)| {
            let alpha = if new[i] - *c > 0.0 {
                self.alpha_rise
            } else {
                self.alpha_decay
            };
            *c = alpha * new[i] + (1.0 - alpha) * (*c);
        })
    }
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

pub fn new_rfft(fft_size: u32) -> Arc<dyn Fft<f64>> {
    FftPlanner::new().plan_fft_forward(fft_size as usize)
}

/**
 * A transformation matrix for mel spectrum
 * mel\_y: the transformation matrix
 * mel\_x: the center frequencies of the mel bands
 */
pub struct MelBank {
    pub x: Array1<f64>,
    pub y: Array2<f64>,
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

fn gaussian_kernel(sigma: f64, order: u32, radius: u32) -> Array1<f64> {
    let exponent_range: Array1<u32> = ndarray::ArrayBase::from_iter(0..order + 1);
    let order = order as usize;
    let x: Array1<i32> = ndarray::ArrayBase::from_iter(-(radius as i32)..(radius + 1) as i32);
    let sigma2 = sigma * sigma;
    let mut phi_x: Array1<f64> = x.map(|v| (-0.5 / sigma2 * v.pow(2) as f64).exp());
    let sum = phi_x.sum();
    phi_x = phi_x / sum;

    if order == 0 {
        return phi_x;
    }

    let mut q = ndarray::Array2::<f64>::zeros((order + 1, 1));
    q[(0, 0)] = 1.;
    let mut d = ndarray::Array2::<f64>::zeros((order + 1, order + 1));
    let mut p = ndarray::Array2::<f64>::zeros((order + 1, order + 1));
    for i in 0..order {
        d[(i, i + 1)] = exponent_range[i + 1] as f64;
        p[(i + 1, i)] = -1. / sigma2;
    }
    let q_deriv = d + p;
    for _ in 0..order {
        q = q_deriv.dot(&q);
    }

    let mut x_pow = Array2::<f64>::zeros((x.shape()[0], exponent_range.shape()[0]));
    for i in 0..x.shape()[0] {
        for j in 0..exponent_range.shape()[0] {
            x_pow[(i, j)] = (x[i].pow(exponent_range[j])) as f64;
        }
    }
    q = x_pow.dot(&q);
    &q.slice(s![.., 0]) * &phi_x
}

fn correlate_1d(arr: &Array2<f64>, kern: &Array1<f64>) -> Array2<f64> {
    let mut output = Array2::zeros((arr.shape()[0], arr.shape()[1]));
    // extend arr by mirroring the ends
    let left_padding = kern.shape()[0] / 2;
    let right_padding = left_padding - 1 + (kern.shape()[0] % 2);
    let right_extension = arr.slice(s![.., arr.shape()[1] - right_padding..; -1]);
    let left_extension = arr.slice(s![.., ..left_padding; -1]);
    let array_extended = ndarray::concatenate![Axis(1), left_extension, *arr, right_extension];
    let kern = kern.slice(s![.., NewAxis]);

    array_extended
        .axis_windows(Axis(1), kern.len())
        .into_iter()
        .enumerate()
        .for_each(|(i, w)| {
            output.slice_mut(s![.., i, NewAxis]).assign(&w.dot(&kern));
        });
    output
}

#[cfg(test)]
mod test {
    use std::fs::File;

    use approx::assert_abs_diff_eq;
    use ndarray::{arr1, arr2};
    use ndarray_npz::NpzReader;

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
        assert_abs_diff_eq!(output, &expected, epsilon = 1e-3);
    }

    #[test]
    fn test_rfft() {
        let mut config = Config::default();
        config.n_fft_bins = 16;

        let dsp = Dsp::new(config, 0.2, 0.2);

        let mut npz_reader = NpzReader::new(File::open("./test/rfft_test.npz").unwrap()).unwrap();

        let input: Array1<f64> = npz_reader.by_name("input.npy").unwrap();
        let expected: Array1<f64> = npz_reader.by_name("expected.npy").unwrap();
        let input2: Array1<f64> = npz_reader.by_name("input2.npy").unwrap();
        let expected2: Array1<f64> = npz_reader.by_name("expected2.npy").unwrap();

        let output = dsp.exec_rfft(&input);

        let epsilon = 1e-3;
        assert_eq!(output.len(), expected.len());
        assert_abs_diff_eq!(output, expected, epsilon = epsilon);
        let output2 = dsp.exec_rfft(&input2);
        assert_abs_diff_eq!(output2, expected2, epsilon = epsilon);
    }

    #[test]
    fn test_correlate_1d() {
        let weights = arr1(&[1., 2., 3., 4.]);
        let input = arr2(&[
            [0., 1., 2., 3., 4.],
            [5., 6., 7., 8., 9.],
            [10., 11., 12., 13., 14.],
            [15., 16., 17., 18., 19.],
            [20., 21., 22., 23., 24.],
        ]);

        let expected_output = arr2(&[
            [5., 11., 20., 30., 36.],
            [55., 61., 70., 80., 86.],
            [105., 111., 120., 130., 136.],
            [155., 161., 170., 180., 186.],
            [205., 211., 220., 230., 236.],
        ]);

        assert_eq!(correlate_1d(&input, &weights), expected_output);
    }

    #[test]
    fn test_gaussian_filter1d() {
        let input = arr2(&[
            [0., 1., 2., 3., 4.],
            [5., 6., 7., 8., 9.],
            [10., 11., 12., 13., 14.],
            [15., 16., 17., 18., 19.],
            [20., 21., 22., 23., 24.],
        ]);

        let expected_output = arr2(&[
            [
                3.72662540e-06,
                1.00000000e+00,
                2.00000000e+00,
                3.00000000e+00,
                3.99999627e+00,
            ],
            [
                5.00000373e+00,
                6.00000000e+00,
                7.00000000e+00,
                8.00000000e+00,
                8.99999627e+00,
            ],
            [
                1.00000037e+01,
                1.10000000e+01,
                1.20000000e+01,
                1.30000000e+01,
                1.39999963e+01,
            ],
            [
                1.50000037e+01,
                1.60000000e+01,
                1.70000000e+01,
                1.80000000e+01,
                1.89999963e+01,
            ],
            [
                2.00000037e+01,
                2.10000000e+01,
                2.20000000e+01,
                2.30000000e+01,
                2.39999963e+01,
            ],
        ]);

        let dsp = Dsp::new(Config::default(), 0.2, 0.2);
        assert_abs_diff_eq!(
            dsp.gaussian_filter1d(&input),
            expected_output,
            epsilon = 1e-5
        );
    }

    #[test]
    fn test_create_gaussian_kernel_higher_order() {
        let expected = arr1(&[
            1.38042650e-83,
            7.76346458e-46,
            4.77362029e-19,
            2.23597524e-03,
            -2.49998137e+01,
            2.23597524e-03,
            4.77362029e-19,
            7.76346458e-46,
            1.38042650e-83,
        ]);

        assert_abs_diff_eq!(gaussian_kernel(0.2, 2, 4), expected, epsilon = 1e-3);
    }

    #[test]
    fn test_create_gaussian_kernel_zero_order() {
        let expected = arr1(&[
            1.38388621e-87,
            1.38633296e-49,
            1.92873547e-22,
            3.72662540e-06,
            9.99992547e-01,
            3.72662540e-06,
            1.92873547e-22,
            1.38633296e-49,
            1.38388621e-87,
        ]);

        assert_abs_diff_eq!(gaussian_kernel(0.2, 0, 4), expected, epsilon = 1e-5);
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
        assert_abs_diff_eq!(output.x, &expected.x, epsilon = epsilon);
        assert_abs_diff_eq!(output.y, &expected.y, epsilon = epsilon);
    }
}
