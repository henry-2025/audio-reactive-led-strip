#include "dsp.h"
#include "config.h"
#include <fftw3.h>
#include <math.h>

extern float hertz_to_melf(float freq);
extern float mel_to_hertzf(float mel);

/**
 * Single exponential filter value. Returns the new value to the stack
 */
float exp_filter_single(float current_val, float new_val, float alpha_decay,
                        float alpha_rise) {
  float alpha = new_val > current_val ? alpha_rise : alpha_decay;
  return alpha * new_val + (1.0 - alpha) * current_val;
}

/**
 * Exponential filter on an array. Modifies current\_val in-place with the
 * updated new\_val
 */
void exp_filter_array(size_t size, float *current_val, float *new_val,
                      float alpha_decay, float alpha_rise) {
  for (size_t i = 0; i < size; i++) {
    float alpha = new_val[i] - current_val[i] > 0.0 ? alpha_rise : alpha_decay;
    current_val[i] = alpha * new_val[i] + (1.0 - alpha) * current_val[i];
  }
}

/**
 * Executes the rfft in-place, yielding the absolute value of the coefficients
 */

rfft new_rfft(size_t fft_size) {
  float *in = (float *)fftwf_malloc(fft_size * sizeof(float));
  fftwf_complex *inter =
      (fftwf_complex *)fftwf_malloc(fft_size * sizeof(fftwf_complex));
  float *out = (float *)fftwf_malloc((fft_size * sizeof(float)) / 2);

  fftwf_plan plan = fftwf_plan_dft_r2c_1d(fft_size, in, inter, FFTW_MEASURE);

  rfft a = {
      .p = plan, .in = in, .inter = inter, .out = out, .fft_size = fft_size};
  return a;
}

void run_rfft(rfft c) {
  fftwf_execute(c.p);
  for (int i = 0; i < c.fft_size / 2; i++) {
    c.out[i] = sqrtf(powf(c.inter[i][0], 2) + powf(c.inter[i][1], 2));
  }
}

void destroy_rfft(rfft c) {
  fftwf_free(c.in);
  fftwf_free(c.out);
  fftwf_free(c.inter);
  fftwf_destroy_plan(c.p);
}

void melfrequencies_mel_filterbank(float freq_min, float freq_max,
                                   float *frequencies_mel,
                                   float *lower_edges_mel,
                                   float *upper_edges_mel,
                                   float *center_frequencies_mel) {
  float mel_max = hertz_to_melf(freq_max);
  float mel_min = hertz_to_melf(freq_min);
  float delta_mel = fabsf(mel_max - mel_min) / (NUM_BANDS + 1.0);
  for (int i = 0; i < NUM_BANDS + 2; i++) {
    frequencies_mel[i] = mel_min + delta_mel * i;
  }
  lower_edges_mel = frequencies_mel;
  upper_edges_mel = frequencies_mel + 2;
  center_frequencies_mel = frequencies_mel + 1;
}

void compute_melmat(uint freq_min, uint freq_max, uint sample_rate,
                    float melmat[NUM_BANDS][N_FFT_BANDS],
                    float freqs[N_FFT_BANDS]) {
  float frequencies_mel[NUM_BANDS + 2];
  float *center_frequencies_mel, *lower_edges_mel, *upper_edges_mel;
  melfrequencies_mel_filterbank(freq_min, freq_max, frequencies_mel,
                                lower_edges_mel, upper_edges_mel,
                                center_frequencies_mel);

  for (int i = 0; i < NUM_BANDS + 2; i++) {
    frequencies_mel[i] = mel_to_hertzf(frequencies_mel[i]);
  }

  for (int i = 0; i < N_FFT_BANDS; i++) {
    freqs[i] = i * (sample_rate / 2.0);
  }

  // melmat = zeros((num_mel_bands, num_fft_bands))
  for (int i = 0; i < NUM_BANDS; i++) {
    int left_slope[NUM_BANDS];
    int right_slope[NUM_BANDS];
    for (int j = 0; j < N_FFT_BANDS; j++) {
      if ((freqs[j] >= lower_edges_mel[j]) ==
          (freqs[j] <= center_frequencies_mel[j])) {
        melmat[i][j] = (freqs[j] - lower_edges_mel[i]) /
                       (center_frequencies_mel[i] - lower_edges_mel[i]);
      }
      if ((freqs[j] >= center_frequencies_mel[j]) ==
          (freqs[j] <= upper_edges_mel[j])) {
        melmat[i][j] = (upper_edges_mel[i] - freqs[j]) /
                       (upper_edges_mel[i] - center_frequencies_mel[i]);
      }
    }
  }
}

void create_mel_bank(size_t size, float mel_y[NUM_BANDS][N_FFT_BANDS],
                     float mel_x[N_FFT_BANDS], int *samples,
                     struct config cfg) {
  *samples = (int)(cfg.mic_rate * cfg.n_rolling_history / (2.0 * cfg.fps));
  compute_melmat(cfg.freq_min, cfg.freq_max, cfg.sample_rate, mel_y, mel_x);
}
