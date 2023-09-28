#ifndef DSP_H
#define DSP_H

#include "config.h"
#include <assert.h>
#include <fftw3.h>
#include <math.h>
#include <stdlib.h>

float exp_filter_single(float current_val, float new_val, float alpha_decay,
                        float alpha_rise);

void exp_filter_array(size_t size, float *current_val, float *new_val,
                      float alpha_decay, float alpha_rise);

typedef struct {
  fftwf_plan p;
  float *in;
  fftwf_complex *inter;
  float *out;
  size_t fft_size;
} rfft;

rfft new_rfft(size_t fft_size);
void run_rfft(rfft c);
void destroy_rfft(rfft c);

inline float hertz_to_melf(float freq) {
  return 2595.0 * log10f(1 + (freq / 700.0));
}
inline float mel_to_hertzf(float mel) {
  return 700.0 * powf(10, mel / 2595.0) - 700;
}
void melfrequencies_mel_filterbank(float freq_min, float freq_max,
                                   float *frequencies_mel,
                                   float *lower_edges_mel,
                                   float *upper_edges_mel,
                                   float *center_frequencies_mel);
void compute_melmat(uint freq_min, uint freq_max, uint sample_rate,
                    float melmat[NUM_BANDS][N_FFT_BANDS],
                    float freqs[N_FFT_BANDS]);

void create_mel_bank(size_t size, float mel_y[NUM_BANDS][N_FFT_BANDS],
                     float mel_x[N_FFT_BANDS], int *samples, struct config cfg);
#endif
