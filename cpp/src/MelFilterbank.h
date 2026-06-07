#pragma once
#include <vector>

// Triangular mel-scale filterbank.
// Transforms an FFT magnitude spectrum into N mel-spaced frequency bands.
// Matches the Python melbank.compute_melmat() implementation exactly.
class MelFilterbank {
public:
    // n_fft_bands: number of FFT magnitude bins fed to apply() (= FFT_BINS_USED)
    MelFilterbank(int n_mel_bands, float freq_min, float freq_max,
                  int n_fft_bands, float sample_rate);

    // input[n_fft_bands_] -> output[n_mel_bands_]
    // Computes mel[i] = sum_j(input[j] * matrix[i][j])
    void apply(const float* input, float* output) const;

    int melBands()  const { return n_mel_bands_; }
    int fftBands()  const { return n_fft_bands_; }

    // Rebuild the filterbank (e.g. when the user changes the frequency range)
    void rebuild(float freq_min, float freq_max);

private:
    int   n_mel_bands_;
    int   n_fft_bands_;
    float sample_rate_;
    float freq_min_, freq_max_;

    // Row-major: matrix_[i * n_fft_bands_ + j] = weight of FFT bin j in mel band i
    std::vector<float> matrix_;

    void build();

    static float hzToMel(float hz);
    static float melToHz(float mel);
};
