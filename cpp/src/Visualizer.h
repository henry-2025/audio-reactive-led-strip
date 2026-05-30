#pragma once
#include "Config.h"
#include "ExpFilter.h"
#include <array>
#include <cstdint>

// Planar RGB frame: pixels[channel][pixel_index], values 0-255.
using PixelFrame = std::array<std::array<uint8_t, Config::N_PIXELS>, 3>;

// Converts mel-scale frequency data into RGB LED values.
// Maintains all inter-frame filter state internally.
// Call process() once per audio frame with the N_FFT_BINS mel values from DSPPipeline.
class Visualizer {
public:
    enum class Effect { Scroll, Energy, Spectrum };

    Visualizer();

    void process(const float* mel, PixelFrame& out);
    void setEffect(Effect e) { effect_ = e; }
    Effect effect() const { return effect_; }
    float hueAngle() const { return hue_angle_; }

private:
    static constexpr int HALF = Config::N_PIXELS / 2; // 67
    static constexpr int N    = Config::N_FFT_BINS;   // 24

    Effect effect_ = Effect::Spectrum;

    // Pixel state shared between Scroll and Energy (matches Python's global `p`).
    // Layout: p_[ch * HALF + px], ch in {0,1,2}.
    float p_[3 * HALF]{};

    // Shared gain normaliser used by Scroll and Energy.
    ExpFilterArray gain_{N, 0.01f, 0.001f, 0.99f};

    // Energy: per-pixel temporal smoothing of the hard 0/255 bar.
    ExpFilterArray p_filt_{3 * HALF, 1.0f, 0.1f, 0.99f};

    // Spectrum: per-channel half-strip filters.
    ExpFilterArray r_filt_      {HALF, 0.01f, 0.2f,  0.99f};
    ExpFilterArray b_filt_      {HALF, 0.01f, 0.1f,  0.5f};
    ExpFilterArray common_mode_ {HALF, 0.01f, 0.99f, 0.01f};
    float prev_spectrum_[HALF]{};

    // Spectrum hue rotation: filtered total mel energy drives the rotation rate.
    // The angle accumulates over time, cycling colours as the music plays.
    ExpFilter hue_energy_{0.0f, 0.01f, 0.98f};
    float     hue_angle_ = 0.0f;

    void doScroll  (const float* mel);
    void doEnergy  (const float* mel);
    void doSpectrum(const float* mel, PixelFrame& out);

    // Mirror p_ into the full-strip output: [p reversed | p].
    void mirrorOut(PixelFrame& out) const;

    static void linearInterp(const float* in, int n_in, float* out, int n_out);
    static uint8_t clamp255(float v);
};
