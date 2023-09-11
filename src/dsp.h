#ifndef DSP_H
#define DSP_H

#include <stdlib.h>
#include <assert.h>

void exp_filter_single(float val, float alpha_decay, float alpha_rise);

void exp_filter_array(size_t size, float *val, float alpha_decay, float alpha_rise);

void rfft(size_t size, float *data, float window);
void fft(size_t size, float *data, float window);
void create_mel_bank();
#endif
