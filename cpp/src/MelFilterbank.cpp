#include "MelFilterbank.h"
#include <cmath>
#include <stdexcept>

float MelFilterbank::hzToMel(float hz) {
    return 2595.0f * std::log10(1.0f + hz / 700.0f);
}

float MelFilterbank::melToHz(float mel) {
    return 700.0f * (std::pow(10.0f, mel / 2595.0f) - 1.0f);
}

MelFilterbank::MelFilterbank(int n_mel_bands, float freq_min, float freq_max,
                             int n_fft_bands, float sample_rate)
    : n_mel_bands_(n_mel_bands)
    , n_fft_bands_(n_fft_bands)
    , sample_rate_(sample_rate)
    , freq_min_(freq_min)
    , freq_max_(freq_max)
    , matrix_(n_mel_bands * n_fft_bands, 0.0f)
{
    if (n_mel_bands <= 0 || n_fft_bands <= 0)
        throw std::invalid_argument("MelFilterbank: band counts must be > 0");
    build();
}

void MelFilterbank::rebuild(float freq_min, float freq_max) {
    freq_min_ = freq_min;
    freq_max_ = freq_max;
    std::fill(matrix_.begin(), matrix_.end(), 0.0f);
    build();
}

// Mirrors melbank.compute_melmat() / melfrequencies_mel_filterbank() exactly.
void MelFilterbank::build() {
    float mel_min = hzToMel(freq_min_);
    float mel_max = hzToMel(freq_max_);
    float delta   = (mel_max - mel_min) / (n_mel_bands_ + 1.0f);

    // Evenly-spaced mel frequencies: n_mel_bands_ + 2 points
    // lower_edges_mel = freqs[0 .. n-1]
    // center_mel      = freqs[1 .. n]
    // upper_edges_mel = freqs[2 .. n+1]
    std::vector<float> mel_freqs(n_mel_bands_ + 2);
    for (int i = 0; i < n_mel_bands_ + 2; ++i)
        mel_freqs[i] = mel_min + delta * i;

    // Linear FFT bin frequencies: 0 .. sample_rate/2
    std::vector<float> fft_freqs(n_fft_bands_);
    for (int j = 0; j < n_fft_bands_; ++j)
        fft_freqs[j] = (sample_rate_ / 2.0f) * j / (n_fft_bands_ - 1);

    for (int i = 0; i < n_mel_bands_; ++i) {
        float lower  = melToHz(mel_freqs[i]);
        float center = melToHz(mel_freqs[i + 1]);
        float upper  = melToHz(mel_freqs[i + 2]);

        float* row = &matrix_[i * n_fft_bands_];

        for (int j = 0; j < n_fft_bands_; ++j) {
            float f = fft_freqs[j];
            // Left slope: [lower, center]
            if (f >= lower && f <= center && center != lower)
                row[j] = (f - lower) / (center - lower);
            // Right slope: (center, upper]
            else if (f > center && f <= upper && upper != center)
                row[j] = (upper - f) / (upper - center);
        }
    }
}

void MelFilterbank::apply(const float* input, float* output) const {
    for (int i = 0; i < n_mel_bands_; ++i) {
        const float* row = &matrix_[i * n_fft_bands_];
        float sum = 0.0f;
        for (int j = 0; j < n_fft_bands_; ++j)
            sum += input[j] * row[j];
        output[i] = sum;
    }
}
