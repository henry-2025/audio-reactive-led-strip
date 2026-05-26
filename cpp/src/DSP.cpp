#include "DSP.h"
#include <cmath>
#include <algorithm>
#include <numeric>
#include <cstring>
#include <stdexcept>

// ── Gaussian filter ───────────────────────────────────────────────────────────

void gaussianFilter1d(const float* in, float* out, int n, float sigma) {
    if (sigma <= 0.0f) {
        std::copy(in, in + n, out);
        return;
    }

    // Truncate kernel at 4 * sigma (matches scipy default truncate=4.0)
    int radius = static_cast<int>(std::ceil(4.0f * sigma));
    int ksize  = 2 * radius + 1;

    std::vector<float> kernel(ksize);
    float sum = 0.0f;
    for (int k = -radius; k <= radius; ++k) {
        float v = std::exp(-0.5f * (k * k) / (sigma * sigma));
        kernel[k + radius] = v;
        sum += v;
    }
    for (float& v : kernel) v /= sum;

    // Reflect boundary: index clamped by mirroring at edges (scipy 'reflect')
    for (int i = 0; i < n; ++i) {
        float acc = 0.0f;
        for (int k = -radius; k <= radius; ++k) {
            int idx = i + k;
            // Reflect: fold back at boundaries
            if (idx < 0)   idx = -idx - 1;
            if (idx >= n)  idx = 2 * n - idx - 1;
            idx = std::clamp(idx, 0, n - 1);
            acc += in[idx] * kernel[k + radius];
        }
        out[i] = acc;
    }
}

void gaussianFilter1dInPlace(float* data, int n, float sigma) {
    std::vector<float> tmp(n);
    gaussianFilter1d(data, tmp.data(), n, sigma);
    std::copy(tmp.begin(), tmp.end(), data);
}

// ── DSPPipeline ───────────────────────────────────────────────────────────────

static void buildHammingWindow(float* out, int n) {
    for (int i = 0; i < n; ++i)
        out[i] = 0.54f - 0.46f * std::cos(2.0f * M_PI * i / (n - 1));
}

DSPPipeline::DSPPipeline()
    : mel_bank_(Config::N_FFT_BINS,
                Config::MIN_FREQUENCY,
                Config::MAX_FREQUENCY,
                Config::FFT_BINS_USED,
                static_cast<float>(Config::MIC_RATE))
{
    buildHammingWindow(hamming_, Config::ROLLING_WINDOW_SIZE);

    // FFTW_ESTIMATE avoids measuring transforms at startup;
    // acceptable because FFT_SIZE is fixed and small.
    plan_ = fftwf_plan_dft_r2c_1d(Config::FFT_SIZE,
                                   fft_in_, fft_out_,
                                   FFTW_ESTIMATE);
    if (!plan_)
        throw std::runtime_error("DSPPipeline: fftwf_plan_dft_r2c_1d failed");

    // Initialise rolling buffer with very small random-ish values, matching
    // the Python: y_roll = np.random.rand(...) / 1e16
    std::fill(rolling_, rolling_ + Config::ROLLING_WINDOW_SIZE, 1e-17f);
}

DSPPipeline::~DSPPipeline() {
    fftwf_destroy_plan(plan_);
}

bool DSPPipeline::update(const float* frame) {
    // Shift rolling window left by one frame, append new frame at the end.
    constexpr int SPF = Config::SAMPLES_PER_FRAME;
    constexpr int RWS = Config::ROLLING_WINDOW_SIZE;
    std::memmove(rolling_, rolling_ + SPF, (RWS - SPF) * sizeof(float));
    std::memcpy(rolling_ + (RWS - SPF), frame, SPF * sizeof(float));

    // Volume gate.
    float vol = 0.0f;
    for (int i = 0; i < RWS; ++i)
        vol = std::max(vol, std::abs(rolling_[i]));
    if (vol < Config::MIN_VOLUME_THRESHOLD)
        return false;

    // Apply Hamming window and zero-pad to FFT_SIZE.
    std::fill(fft_in_, fft_in_ + Config::FFT_SIZE, 0.0f);
    for (int i = 0; i < RWS; ++i)
        fft_in_[i] = rolling_[i] * hamming_[i];

    // Execute real-to-complex FFT.
    fftwf_execute(plan_);

    // Magnitude of the first FFT_BINS_USED bins (Python: YS = |rfft(y_padded)|[:N//2])
    for (int i = 0; i < Config::FFT_BINS_USED; ++i) {
        float re = fft_out_[i][0];
        float im = fft_out_[i][1];
        mag_[i] = std::sqrt(re * re + im * im);
    }

    // Apply mel filterbank: mel[i] = sum_j(mag[j] * mel_matrix[i][j])
    mel_bank_.apply(mag_, mel_raw_);

    // Perceptual loudness: square the mel magnitudes.
    for (int i = 0; i < Config::N_FFT_BINS; ++i)
        mel_raw_[i] *= mel_raw_[i];

    // Gain normalisation: track the max of a Gaussian-smoothed copy.
    float smoothed[Config::N_FFT_BINS];
    gaussianFilter1d(mel_raw_, smoothed, Config::N_FFT_BINS, 1.0f);
    float max_val = *std::max_element(smoothed, smoothed + Config::N_FFT_BINS);
    mel_gain_.update(max_val);

    float normalised[Config::N_FFT_BINS];
    for (int i = 0; i < Config::N_FFT_BINS; ++i)
        normalised[i] = mel_raw_[i] / mel_gain_.value;

    // Temporal smoothing.
    mel_smoothing_.update(normalised, Config::N_FFT_BINS);

    return true;
}

void DSPPipeline::setFrequencyRange(float min_hz, float max_hz) {
    mel_bank_.rebuild(min_hz, max_hz);
}
