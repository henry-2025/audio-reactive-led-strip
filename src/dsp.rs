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
    mel_gain: ExpFilterArr<Ix1>,
    mel_smoothing: ExpFilterArr<Ix1>,
    fft: Arc<dyn Fft<f64>>,
    config: Config,
}

#[derive(Clone)]
pub enum Preset {
    Scroll,
    Power,
    Spectrum,
}

impl Dsp {
    pub fn new(config: Config) -> Self {
        Self {
            gain: ExpFilterArr::<Ix1>::new(config.n_mel_bands as usize, 0.01, 0.2, 0.2),
            p_filt: ExpFilterArr::<Ix2>::new(
                (config.n_points / 2 + config.n_points % 2) as usize,
                1.,
                0.99,
                0.1,
            ),
            common_mode: ExpFilterArr::<Ix1>::new(
                (config.n_mel_bands / 2) as usize,
                0.01,
                0.99,
                0.01,
            ),
            r_filt: ExpFilterArr::<Ix1>::new((config.n_points / 2) as usize, 0.01, 0.2, 0.99),
            g_filt: ExpFilterArr::<Ix1>::new((config.n_points / 2) as usize, 0.01, 0.05, 0.3),
            b_filt: ExpFilterArr::<Ix1>::new((config.n_points / 2) as usize, 0.01, 0.1, 0.5),
            prev_spectrum: Array1::zeros(config.n_mel_bands as usize),
            gaussian_kernel1: gaussian_kernel(0.2, 0, 1), // TODO: determine whether radius 1 is what we want
            gaussian_kernel2: gaussian_kernel(0.4, 0, 1), // TODO: determine whether radius 1 is what we want
            mel_bank: create_mel_bank(
                config.mic_rate,
                config.n_fft_bins / 2,
                config.n_mel_bands,
                config.min_freq_hz,
                config.max_freq_hz,
            ),
            mel_gain: ExpFilterArr::<Ix1>::new(config.n_mel_bands as usize, 0.1, 0.01, 0.99),
            mel_smoothing: ExpFilterArr::<Ix1>::new(config.n_mel_bands as usize, 0.1, 0.5, 0.99),
            fft: new_rfft(config.n_fft_bins),
            config,
        }
    }
    pub fn apply_transform_inplace(&mut self, preset: Preset, display_values: &mut Array2<f64>) {
        match preset {
            Preset::Scroll => self.visualize_scroll(display_values),
            Preset::Power => self.visualize_power(display_values),
            Preset::Spectrum => self.visualize_spectrum(display_values),
        };
    }

    fn visualize_scroll(&mut self, display_values: &mut Array2<f64>) {
        let mut display_slice = display_values
            .slice(s![(self.config.n_points / 2) as usize.., ..])
            .to_owned();
        let mut y = self.mel_smoothing.current.clone();
        // y = y**2.0
        y.map_inplace(|x| *x = x.powi(2));
        // update gain
        self.gain.update(&y);
        // y /= gain.value
        // y *= 255
        y.zip_mut_with(&self.gain.current, |y, g| *y = 255.0 * (*y) / g);

        // scrolling effect
        // p[1:, :] = p[:-1, :]
        // p *= 0.98
        for i in 1..display_slice.shape()[0] - 1 {
            let left_pixels = display_slice.slice(s![i - 1, ..]).to_owned() * 0.98;
            display_slice.slice_mut(s![i, ..]).assign(&left_pixels);
        }
        // apply gaussian filter
        let mut filter_display_buffer = correlate_1d(&display_slice, &self.gaussian_kernel1);

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
        display_values.assign(&ndarray::concatenate![
            Axis(0),
            filter_display_buffer.slice(s![(self.config.n_points % 2) as usize..,..;-1]),
            filter_display_buffer,
        ]);
    }
    fn visualize_power(&mut self, display_values: &mut Array2<f64>) {
        let mut y = self.mel_smoothing.current.clone();
        self.gain.update(&y);
        let mut display_slice = display_values
            .slice(s![(self.config.n_points / 2) as usize.., ..])
            .to_owned();

        // y /= gain.value
        // y *= float(config.n_pixels // 2) - 1)
        y.zip_mut_with(&self.gain.current, |y, g| {
            *y *= ((self.config.n_points / 2) - 1) as f64 / g;
        });

        // map color channels according to energy in different frequency bands
        let scale = 0.9;
        for i in 0..3 {
            let s = y
                .slice(s![i * y.shape()[0] / 3..(i + 1) * y.shape()[0] / 3])
                .map(|x| x.powf(scale));
            let mean = s.mean().unwrap() as usize;
            println!("{}", mean);
            display_slice.slice_mut(s![..mean, i]).fill(255.0);
            display_slice.slice_mut(s![mean.., i]).fill(0.0);
        }

        self.p_filt.update(&display_slice);
        display_slice.map_inplace(|x| {
            *x = x.round();
        });
        display_slice.assign(&correlate_1d(&display_slice, &self.gaussian_kernel2));

        display_values.assign(&ndarray::concatenate![
            Axis(0),
            display_slice.slice(s![(self.config.n_points % 2) as usize..,..;-1]),
            display_slice
        ]);
    }
    fn visualize_spectrum(&mut self, display_buffer: &mut Array2<f64>) {
        // TODO: need to do interpolation up here?
        let y = self.mel_smoothing.current.clone();
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

        display_buffer.assign(&ndarray::stack![Axis(0), r, g, b]);
    }

    pub fn exec_rfft(&self, buffer: &Array1<f64>) -> Array1<f64> {
        let mut complex_buffer: Array1<Complex64> =
            buffer.iter().map(|x| Complex64::new(*x, 0.0)).collect();

        self.fft.process(complex_buffer.as_slice_mut().unwrap());
        complex_buffer
            .mapv(Complex64::abs)
            .slice(s![..buffer.len() / 2])
            .to_owned()
    }

    pub fn gain_and_smooth(&mut self, mel: &mut Array1<f64>) {
        mel.map_mut(|x| *x = x.powi(2));
        let filtered_mel = self.gaussian_filter1d_single(mel);
        self.mel_gain.update(&filtered_mel);
        mel.zip_mut_with(&self.mel_gain.current, |m, g| *m /= g);
        self.mel_smoothing.update(mel);
    }

    pub fn get_mel_repr(&self, audio: &Array1<f64>) -> Array1<f64> {
        self.mel_bank.y.dot(audio)
    }

    fn gaussian_filter1d(&self, input: &Array2<f64>) -> Array2<f64> {
        correlate_1d(input, &self.gaussian_kernel1.slice(s![..;-1]).to_owned())
    }

    fn gaussian_filter1d_single(&self, input: &Array1<f64>) -> Array1<f64> {
        correlate_1d_single(input, &self.gaussian_kernel1.slice(s![..;-1]).to_owned())
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
    pub fn new(size: usize, init: f64, alpha_rise: f64, alpha_decay: f64) -> Self {
        Self {
            current: Array::<f64, Ix2>::ones((size, 3)) * init,
            alpha_rise,
            alpha_decay,
        }
    }
    pub fn update(&mut self, new: &Array2<f64>) {
        assert_eq!(self.current.shape(), new.shape());
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
    pub fn new(size: usize, init: f64, alpha_rise: f64, alpha_decay: f64) -> Self {
        Self {
            current: Array::<f64, Ix1>::ones(size) * init,
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
    n_fft_bins: u32,
    n_mel_bands: u32,
    min_freq_hz: u32,
    max_freq_hz: u32,
) -> MelBank {
    // generate centerfrequencies and band eges for a mel filter bank
    let max_freq_mel = hertz_to_mel(max_freq_hz as f64);
    let min_freq_mel = hertz_to_mel(min_freq_hz as f64);
    let delta_mel = (max_freq_mel - min_freq_mel).abs() / (n_mel_bands as f64 + 1.0);
    let frequencies_mel =
        Array1::from_iter((0..n_mel_bands + 2).map(|i| i as f64)) * delta_mel + min_freq_mel;
    let frequencies_hz = frequencies_mel.map(|mel| mel_to_hertz(*mel));

    // build the mel input scale and transformation matrix
    let mel_x = ndarray::Array1::linspace(0., mic_rate as f64 / 2.0, n_fft_bins as usize);
    let mel_y = Array2::from_shape_fn((n_mel_bands as usize, n_fft_bins as usize), |(i, j)| {
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

fn correlate_1d_single(arr: &Array1<f64>, kern: &Array1<f64>) -> Array1<f64> {
    let mut output = Array1::<f64>::zeros(arr.shape()[0]);
    // extend arr by mirroring the ends
    let left_padding = kern.shape()[0] / 2;
    let right_padding = left_padding - 1 + (kern.shape()[0] % 2);
    let right_extension = arr.slice(s![arr.shape()[0] - right_padding..; -1]);
    let left_extension = arr.slice(s![..left_padding; -1]);
    let array_extended = ndarray::concatenate![Axis(0), left_extension, *arr, right_extension];
    let kern = kern.slice(s![.., NewAxis]);

    array_extended
        .axis_windows(Axis(0), kern.len())
        .into_iter()
        .enumerate()
        .for_each(|(i, w)| {
            output[i] = w.dot(&kern)[0];
        });
    output
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
mod test_dsp_functions {
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

        let dsp = Dsp::new(config);

        let input1 = arr1(&[
            1.18550208,
            0.24397273,
            -0.32150586,
            0.87697932,
            -1.48763549,
            0.61344813,
            -0.43050085,
            -0.10378538,
            1.01909004,
            0.48247267,
            0.52174026,
            -1.87953681,
            -0.56927693,
            -0.41475547,
            0.7978689,
            0.59856428,
        ]);
        let expected1 = arr1(&[
            1.13264162, 1.70423055, 5.7569633, 3.24484748, 1.49317957, 2.20490158, 2.87665801,
            5.55807008,
        ]);
        let input2 = arr1(&[
            -0.68607134,
            0.26019531,
            0.26886673,
            1.14278013,
            -2.13157004,
            -1.16983613,
            -0.23719303,
            1.57163556,
            0.8962407,
            0.61271303,
            0.2957349,
            -0.20352925,
            0.52946358,
            1.06310262,
            -0.43305559,
            0.32494922,
        ]);
        let expected2 = arr1(&[
            2.1044264, 3.53903493, 3.41887479, 6.64262417, 2.43680902, 2.97127958, 1.29694465,
            2.63174746,
        ]);

        let epsilon = 1e-3;
        let output1 = dsp.exec_rfft(&input1);
        assert_abs_diff_eq!(output1, expected1, epsilon = epsilon);
        let output2 = dsp.exec_rfft(&input2);
        assert_abs_diff_eq!(output2, expected2, epsilon = epsilon);
    }

    #[test]
    fn test_get_mel_repr() {
        let mut config = Config::default();
        config.n_fft_bins = 16;
        config.n_mel_bands = 8;
        let dsp = Dsp::new(config);

        let input = arr1(&[
            0.59944508, 0.35953482, 0.43607555, 1.81651546, 0.05219176, 0.06467918, 0.91489904,
            0.32199603, 0.24770591, 1.36049556, 0.3612345, 1.24795475, 0.63443764, 1.6687458,
            1.33319364, 0.55696517,
        ]);

        let expected = arr1(&[
            0., 0.00314753, 0.35638729, 0.12078571, 0.51270242, 1.63282723, 0.07639316, 1.11434329,
        ]);
        assert_abs_diff_eq!(dsp.get_mel_repr(&input), expected, epsilon = 1e-5);
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

        let dsp = Dsp::new(Config::default());
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
        let output = create_mel_bank(44100, 735, 24, 200, 12000);
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

#[cfg(test)]
mod test_display_funcs {
    use ndarray::{arr1, arr2};

    use crate::config::Config;

    use super::Dsp;

    const DISPLAY_BUFFER: [[f64; 3]; 100] = [
        [-1.08949971e+02, -5.30869048e+00, -1.74564589e+02],
        [2.31932428e+02, 1.17315702e+02, 1.55038034e+02],
        [3.40845550e+02, -3.18216758e+02, -3.09416996e+02],
        [-2.44578309e+02, -2.68573537e+01, -5.40130118e+00],
        [2.45196171e+02, 5.65321646e+02, 5.42808600e+02],
        [-4.23768787e+02, -2.81760746e+02, 2.78457153e+02],
        [-9.02015533e+00, 1.40348236e+01, 5.89245229e+02],
        [-2.15520962e+01, 3.15077887e+02, 3.61884907e+02],
        [-1.11698264e+02, -1.72694543e+02, -3.30300435e+02],
        [-3.42264506e+01, -2.28585370e+02, -3.66265571e+02],
        [1.29799151e+02, -1.59657751e+02, -5.22719342e+00],
        [2.12575707e+02, -1.12952190e-01, -3.17804843e+02],
        [-2.59266300e+01, -1.02977640e+02, 1.48567398e+02],
        [-3.61084299e+02, -1.17357717e+02, -3.13160337e+02],
        [9.99889426e+00, 5.50666968e+02, -2.01592867e+02],
        [1.10085891e+02, -4.77823158e+01, 6.24517165e+01],
        [-4.76305251e+01, 1.38271836e+01, 1.49728584e+02],
        [5.16825438e+01, -3.46112414e+02, 4.83658919e+01],
        [2.60279333e+02, 3.82265531e+02, 1.98615921e+02],
        [2.33315691e+02, -1.17221160e+02, -5.57153758e+00],
        [-1.83315822e+02, 3.12558109e+02, -6.24719916e+01],
        [-1.32235483e+01, -3.90467041e+02, -1.06513534e+02],
        [1.47265314e+02, -5.19503169e+02, 9.50016795e+01],
        [-2.83914526e+02, 3.76718095e+02, 1.80892859e+02],
        [-2.79551197e+02, -1.25373859e+02, 3.12206166e+02],
        [-8.44318338e+01, -2.57606145e+02, -9.32259777e+01],
        [-3.17466651e+02, 1.56568507e+02, -4.15686187e+02],
        [3.34647781e+02, -1.98365053e+02, 1.66440353e+02],
        [-1.22554371e+02, 3.46763587e+02, 2.46206729e+02],
        [1.98367618e+02, 3.07571119e+02, 5.14097132e+02],
        [1.76172411e+02, 4.01918695e+02, 7.49419315e+01],
        [-2.23103342e+02, 1.80646683e+02, 3.34275845e+02],
        [-4.69634823e+01, -1.23643398e+02, -2.53233909e+02],
        [-3.60247729e+01, -1.71768303e+02, 1.74968613e+02],
        [-8.15200452e+01, -4.65784075e+00, -2.39485926e+02],
        [-1.88298741e+02, 1.29872838e+02, -7.76395805e+01],
        [1.25998630e+02, 2.06537709e+02, 9.81546400e+01],
        [-4.88010353e+02, -1.13160363e+02, -2.23262153e+02],
        [-1.45017130e+02, 7.67755532e+01, -3.89884965e+02],
        [2.18983688e+02, 1.25071692e+01, -5.43962318e+02],
        [-1.98741995e+02, -6.79412760e+02, -1.79860016e+02],
        [1.11628157e+02, -2.37014816e+02, -3.41517415e+02],
        [1.60923704e+02, -1.03744119e+02, 1.37318105e+02],
        [1.61961121e+02, 1.42386436e+02, 3.42672199e+00],
        [-3.89630197e+01, -3.48360137e+02, 6.86495806e+00],
        [-2.12653591e+01, -4.91854372e+02, -2.03910611e+02],
        [-2.31509778e+02, 1.48418124e+02, 7.12750441e+00],
        [-1.67444500e+02, 3.48706472e+02, 4.44720773e+02],
        [4.89632474e+02, -9.81283608e+01, -2.32176957e+02],
        [-9.29926175e+00, 3.86534419e+01, 2.99570988e+02],
        [-2.42068345e+01, -3.99373402e+02, 1.84931369e+02],
        [-7.97023474e+01, 2.23948089e+02, -2.23476971e+02],
        [5.33270692e+01, -1.48345222e+02, 2.88123193e+02],
        [-7.66611668e+01, -3.72180500e+02, 1.38321566e+01],
        [7.64522817e+01, -6.07691676e+02, 3.51526153e+02],
        [2.54104422e+02, -4.42216123e+02, -5.24340367e+01],
        [2.49063705e+02, -3.57611568e+02, 7.35101030e+01],
        [-3.85798444e+02, 1.63290074e+02, -1.16279859e+02],
        [9.45318152e+01, -3.58598872e+02, -1.16992960e+02],
        [4.55779401e+02, -1.10170664e+02, 1.90205854e+02],
        [-9.74662247e+01, -8.45089017e+01, 1.03082493e+01],
        [-2.02995170e+02, 2.00054254e+02, -1.76901832e+02],
        [-3.33814516e+02, -3.04943374e+02, 4.97433852e+02],
        [7.90266283e+01, 2.80900580e+02, -2.02667604e+02],
        [6.62625395e+01, 2.27077982e+02, -2.02516894e+02],
        [2.55556691e+02, -1.70413025e+02, 3.33482679e+02],
        [-1.89671164e+02, -5.81289492e+01, -1.99068193e+02],
        [-1.74626794e+02, 1.88466734e+02, 9.23585875e+01],
        [1.38243740e+02, 2.05015547e+02, 2.53868377e+02],
        [2.16785256e+02, 3.08508105e+02, 4.68832915e+02],
        [3.19991748e+02, -4.81723494e+02, -9.16664777e+01],
        [-9.32613346e+01, 5.22888923e+02, -6.65834693e+01],
        [-8.72062898e+01, -3.96691328e+02, -2.43345372e+02],
        [-2.46706575e+00, 7.89413001e+01, 2.88539166e+02],
        [1.52874782e+02, 1.53288824e+02, 3.47949468e+02],
        [3.47329746e+02, -3.60667722e+02, 4.98384650e+01],
        [-2.10882291e+02, 7.85321546e+01, -1.49610006e+02],
        [6.16809406e+01, 2.93014774e+02, 2.25599672e+02],
        [-3.58883586e+02, 4.60202727e+01, -4.77182282e+02],
        [-1.36557743e+02, -3.38424442e+02, 4.55004598e+02],
        [-3.21954046e+02, 1.83829667e+02, 2.01558314e+02],
        [2.51412572e+02, -5.01929926e+01, 1.34109301e+02],
        [4.53889884e+02, -3.80448012e+02, -3.64767677e+02],
        [-7.89401254e+02, 1.27223985e+02, -4.21406382e+02],
        [-1.21713729e+02, -2.42896501e+02, 1.21550536e+02],
        [1.46463476e+02, -1.00829442e+02, -1.31013695e+02],
        [4.45776840e+02, 2.47257226e+02, 1.67193090e+02],
        [7.58069672e+01, 4.84839355e+02, 1.20536402e+02],
        [-8.50984508e+01, 2.80408166e+02, -3.33367812e+01],
        [-4.83238880e+01, 3.51243384e+01, -1.34652678e+02],
        [4.15788831e+02, 2.55128623e+02, -3.98582931e+02],
        [-3.08501499e+01, 6.88404424e+01, -8.64489027e+01],
        [3.58082669e+01, -8.25801493e+01, 8.83812354e+01],
        [-9.68621230e+01, -3.21400746e+02, 3.38933708e+02],
        [5.65492605e+01, 2.86591721e+02, -2.05765154e+02],
        [-5.00486693e+02, 2.51245004e+02, -4.28927168e+02],
        [3.46071889e+02, -2.86809823e+02, -2.57552950e+02],
        [4.97072146e+02, -1.34014087e+02, -4.16787547e+02],
        [-1.14659894e+02, -3.17276346e+02, -1.41908578e+02],
        [-2.31363675e+02, 3.33496981e+02, -3.29107139e+02],
    ];

    const MEL_UPDATE: [f64; 16] = [
        0.59944508, 0.35953482, 0.43607555, 1.81651546, 0.05219176, 0.06467918, 0.91489904,
        0.32199603, 0.24770591, 1.36049556, 0.3612345, 1.24795475, 0.63443764, 1.6687458,
        1.33319364, 0.55696517,
    ];

    #[test]
    fn test_display_scroll() {
        let mut display_buffer = arr2(&DISPLAY_BUFFER);
        let mut config = Config::default();
        config.n_points = display_buffer.shape()[0] as u8;
        config.n_mel_bands = 16;

        let mut dsp = Dsp::new(config);
        dsp.gain_and_smooth(&mut arr1(&MEL_UPDATE));

        dsp.apply_transform_inplace(super::Preset::Scroll, &mut display_buffer);
    }
}
