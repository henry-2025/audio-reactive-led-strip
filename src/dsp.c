#include "dsp.h"
#include "config.h"
#include <cblas.h>
#include <fftw3.h>
#include <math.h>
#include <stdlib.h>

extern double hertz_to_mel(double freq);
extern double mel_to_hertz(double mel);

/**
 * Single exponential filter value. Returns the new value to the stack
 */
double exp_filter_single(double current_val, double new_val, double alpha_decay,
                         double alpha_rise) {
  double alpha = new_val > current_val ? alpha_rise : alpha_decay;
  return alpha * new_val + (1.0 - alpha) * current_val;
}

/**
 * Exponential filter on an array. Modifies current\_val in-place with the
 * updated new\_val
 */
void exp_filter_array(size_t size, double *current_val, double *new_val,
                      double alpha_decay, double alpha_rise) {
  for (size_t i = 0; i < size; i++) {
    double alpha = new_val[i] - current_val[i] > 0.0 ? alpha_rise : alpha_decay;
    current_val[i] = alpha * new_val[i] + (1.0 - alpha) * current_val[i];
  }
}

/**
 * Executes the rfft in-place, yielding the absolute value of the coefficients
 */

rfft new_rfft(size_t fft_size) {
  double *in = (double *)fftw_malloc(fft_size * sizeof(double));
  fftw_complex *inter =
      (fftw_complex *)fftw_malloc(fft_size * sizeof(fftw_complex));
  double *out = (double *)fftw_malloc((fft_size * sizeof(double)) / 2);

  fftw_plan plan = fftw_plan_dft_r2c_1d(fft_size, in, inter, FFTW_MEASURE);

  rfft a = {
      .p = plan, .in = in, .inter = inter, .out = out, .fft_size = fft_size};
  return a;
}

void run_rfft(rfft c) {
  fftw_execute(c.p);
  for (int i = 0; i < c.fft_size / 2; i++) {
    c.out[i] = sqrtf(powf(c.inter[i][0], 2) + powf(c.inter[i][1], 2));
  }
}

void destroy_rfft(rfft c) {
  fftw_free(c.in);
  fftw_free(c.out);
  fftw_free(c.inter);
  fftw_destroy_plan(c.p);
}

// make sure to deallocate this value when finished
double *melfrequencies_mel_filterbank(size_t num_mel_bands, double freq_min,
                                      double freq_max, size_t n_fft_bands) {
  double mel_max = hertz_to_mel(freq_max);
  double mel_min = hertz_to_mel(freq_min);
  double delta_mel = fabs(mel_max - mel_min) / (num_mel_bands + 1.0);
  double *frequencies_mel = malloc((num_mel_bands + 2) * sizeof(double));
  for (size_t i = 0; i < num_mel_bands + 2; i++) {
    frequencies_mel[i] = i * delta_mel + mel_min;
  }
  return frequencies_mel;
}

void compute_melmat(double *mel_x, double *mel_y, size_t n_mel_bands,
                    size_t min_freq, size_t max_freq, size_t n_fft_bands,
                    size_t sample_rate) {

  double *center_frequencies_mel, *lower_edges_mel, *upper_edges_mel;
  lower_edges_mel = melfrequencies_mel_filterbank(n_mel_bands, min_freq,
                                                  max_freq, n_fft_bands);
  center_frequencies_mel = lower_edges_mel + 1;
  upper_edges_mel = lower_edges_mel + 2;

  for (size_t i = 0; i < n_mel_bands + 2; i++) {
    lower_edges_mel[i] = mel_to_hertz(lower_edges_mel[i]);
  }

  // setup linear frequency space
  double step = sample_rate / 2.0 / (n_fft_bands - 1);
  for (size_t i = 1; i < n_fft_bands; i++) {
    mel_x[i] = mel_x[i-1] + step;
  }

  for (size_t i = 0; i < n_mel_bands; i++) {
    float lower = lower_edges_mel[i];
    float center = center_frequencies_mel[i];
    float upper = upper_edges_mel[i];
    for (size_t j = 0; j < n_fft_bands; j++) {
      if ((mel_x[j] >= lower) == (mel_x[j] <= center)) {
        mel_y[i * n_fft_bands + j] = (mel_x[j] - lower) / (center - lower);
      }
      if ((mel_x[j] >= center) == (mel_x[j] <= upper)) {
        mel_y[i * n_fft_bands + j] = (upper - mel_x[j]) / (upper - center);
      }
    }
  }
  // CAUTION: do this free here
  free(lower_edges_mel);
}

mel_bank create_mel_bank(size_t mic_rate, size_t n_rolling_history, size_t fps,
                         size_t n_fft_bins, size_t min_freq, size_t max_freq) {

  // allocate arrays
  size_t samples = (size_t)(mic_rate * n_rolling_history / (2.0 * fps));
  double *melmat = malloc(n_fft_bins * samples * sizeof(double));
  double *freqs = malloc(samples * sizeof(double));

  compute_melmat(freqs, melmat, n_fft_bins, min_freq, max_freq, samples,
                 mic_rate);

  mel_bank ret = {.mel_x = freqs, .mel_y = melmat};
  return ret;
}

void destroy_mel_bank(mel_bank b) {
  free(b.mel_x);
  free(b.mel_y);
}
