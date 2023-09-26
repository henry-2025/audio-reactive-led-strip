#include "dsp.h"
#include "config.h"
#include <fftw3.h>
#include <math.h>

extern float hertz_to_melf(float freq);
extern float mel_to_hertzf(float mel);

float exp_filter_single(float current_val, float new_val, float alpha_decay,
                        float alpha_rise) {
  float alpha = new_val > current_val ? alpha_rise : alpha_decay;
  return alpha * new_val + (1.0 - alpha) * current_val;
}

void exp_filter_array(size_t size, float *current_val, float *new_val,
                      float alpha_decay, float alpha_rise) {
  for (size_t i = 0; i < size; i++) {
    float alpha = new_val[i] - current_val[i] > 0.0 ? alpha_rise : alpha_decay;
    new_val[i] = alpha * current_val[i] + (1.0 - alpha) * current_val[i];
  }
}

fftwf_plan create_rfft_plan(size_t size, float *input, float *output) {
  return fftwf_plan_r2r_1d(size, input, output, FFTW_REDFT10,
                           FFTW_DESTROY_INPUT);
}

fftwf_plan create_fft_plan(size_t size, float *input, fftwf_complex *output) {
  return fftwf_plan_dft_r2c_1d(size, input, output, FFTW_DESTROY_INPUT);
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
                     float mel_x[N_FFT_BANDS], int *samples, struct config cfg) {
  *samples = (int)(cfg.mic_rate * cfg.n_rolling_history / (2.0 * cfg.fps));
  compute_melmat(cfg.freq_min, cfg.freq_max, cfg.sample_rate, mel_y, mel_x);
}
