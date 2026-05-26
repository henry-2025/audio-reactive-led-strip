#pragma once
#include "Config.h"
#include "ExpFilter.h"
#include "MelFilterbank.h"
#include <array>
#include <fftw3.h>

// Applies a 1-D Gaussian blur (scipy.ndimage.gaussian_filter1d equivalent).
// Uses reflect boundary conditions and a kernel truncated at 4*sigma.
void gaussianFilter1d(const float* in, float* out, int n, float sigma);

// In-place variant.
void gaussianFilter1dInPlace(float* data, int n, float sigma);

// ── DSPPipeline ───────────────────────────────────────────────────────────────
//
// Maintains all per-session DSP state and converts raw audio frames into
// mel-scale frequency bands ready for the visualizer.
//
// Call update() once per audio frame. It returns false when the volume is
// below MIN_VOLUME_THRESHOLD (LEDs should be blanked in that case).
//
// The mel output (N_FFT_BINS values in [0, 1]) is available via mel().
class DSPPipeline {
public:
    DSPPipeline();
    ~DSPPipeline();

    // Process one frame of SAMPLES_PER_FRAME normalised float samples.
    // Returns false if volume is below threshold.
    bool update(const float* frame);

    // Gain-normalised, temporally-smoothed mel output — valid after update().
    const float* mel() const { return mel_smoothing_.value.data(); }

    // Unsmoothed mel output (used for the GUI plot via fft_plot_filter).
    const float* melRaw() const { return mel_raw_; }

    // Rebuild mel filterbank after a frequency range change.
    void setFrequencyRange(float min_hz, float max_hz);

private:
    // Rolling window of audio (ROLLING_WINDOW_SIZE = 1470 samples).
    // Oldest frame is discarded on each update() call.
    float rolling_[Config::ROLLING_WINDOW_SIZE]{};

    // Pre-computed Hamming window matching the rolling window length.
    float hamming_[Config::ROLLING_WINDOW_SIZE];

    // FFTW plan and buffers (float precision).
    float         fft_in_[Config::FFT_SIZE]{};
    fftwf_complex fft_out_[(Config::FFT_SIZE / 2) + 1]{};
    fftwf_plan    plan_;

    // FFT magnitude for the first FFT_BINS_USED bins.
    float mag_[Config::FFT_BINS_USED]{};

    // Mel output buffers.
    float mel_raw_[Config::N_FFT_BINS]{};

    // Filters — match Python state exactly.
    ExpFilter      mel_gain_{0.1f, 0.01f, 0.99f};      // scalar, tracks max of smoothed mel
    ExpFilterArray mel_smoothing_{Config::N_FFT_BINS, 0.1f, 0.5f, 0.99f};

    MelFilterbank  mel_bank_;
};
