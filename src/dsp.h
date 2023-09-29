#ifndef DSP_H
#define DSP_H

#include "config.h"
#include <assert.h>
#include <fftw3.h>
#include <math.h>
#include <stdlib.h>

double exp_filter_single(double current_val, double new_val, double alpha_decay,
                        double alpha_rise);

void exp_filter_array(size_t size, double *current_val, double *new_val,
                      double alpha_decay, double alpha_rise);

typedef struct {
  fftw_plan p;
  double *in;
  fftw_complex *inter;
  double *out;
  size_t fft_size;
} rfft;

rfft new_rfft(size_t fft_size);
void run_rfft(rfft c);
void destroy_rfft(rfft c);

inline double hertz_to_mel(double freq) {
  return 2595.0 * log10(1 + (freq / 700.0));
}
inline double mel_to_hertz(double mel) {
  return 700.0 * pow(10, mel / 2595.0) - 700;
}
double *melfrequencies_mel_filterbank(size_t num_mel_bands, double freq_min,
                                     double freq_max, size_t n_fft_bands);

/**
 * A transformation matrix for mel spectrum
 * mel\_y: the transformation matrix
 * mel\_x: the center frequencies of the mel bands
 */
typedef struct {
  double *mel_x;
  double *mel_y;
} mel_bank;

mel_bank create_mel_bank(size_t mic_rate, size_t n_rolling_history,
                         size_t fps, size_t n_fft_bins, size_t min_freq,
                         size_t max_freq);

void destroy_mel_bank(mel_bank b);
#endif
