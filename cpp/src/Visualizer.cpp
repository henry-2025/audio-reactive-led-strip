#include "Visualizer.h"
#include "DSP.h"
#include <algorithm>
#include <cmath>
#include <cstring>

Visualizer::Visualizer() {
    std::fill(p_, p_ + 3 * HALF, 1.0f);
    std::fill(prev_spectrum_, prev_spectrum_ + HALF, 0.01f);
}

uint8_t Visualizer::clamp255(float v) {
    return static_cast<uint8_t>(std::clamp(v, 0.0f, 255.0f));
}

void Visualizer::linearInterp(const float* in, int n_in, float* out, int n_out) {
    for (int i = 0; i < n_out; ++i) {
        float t    = static_cast<float>(i) / (n_out - 1) * (n_in - 1);
        int   lo   = static_cast<int>(t);
        int   hi   = std::min(lo + 1, n_in - 1);
        float frac = t - lo;
        out[i]     = in[lo] * (1.0f - frac) + in[hi] * frac;
    }
}

// Mirror p_ (3 x HALF planar) into the full N_PIXELS output.
// Python: np.concatenate((p[:, ::-1], p), axis=1)
// Produces: [p[HALF-1]..p[0] | p[0]..p[HALF-1]]
void Visualizer::mirrorOut(PixelFrame& out) const {
    for (int ch = 0; ch < 3; ++ch) {
        const float* row = p_ + ch * HALF;
        for (int i = 0; i < HALF; ++i) {
            uint8_t v          = clamp255(row[i]);
            out[ch][HALF-1-i]  = v;
            out[ch][HALF + i]  = v;
        }
    }
}

void Visualizer::process(const float* mel, PixelFrame& out) {
    switch (effect_) {
        case Effect::Scroll:   doScroll(mel);         mirrorOut(out); break;
        case Effect::Energy:   doEnergy(mel);         mirrorOut(out); break;
        case Effect::Spectrum: doSpectrum(mel, out);                  break;
    }
}

// ── Scroll ────────────────────────────────────────────────────────────────────

void Visualizer::doScroll(const float* mel) {
    // Square mel values then gain-normalise and scale to [0, 255].
    float y[N];
    for (int i = 0; i < N; ++i) y[i] = mel[i] * mel[i];
    gain_.update(y, N);
    for (int i = 0; i < N; ++i) y[i] = y[i] / std::max(gain_.value[i], 1e-6f) * 255.0f;

    // One colour per frequency third.
    constexpr int T = N / 3;
    float r = *std::max_element(y,       y + T);
    float g = *std::max_element(y + T,   y + 2*T);
    float b = *std::max_element(y + 2*T, y + N);

    // Shift columns right by one — existing colours scroll outward.
    for (int ch = 0; ch < 3; ++ch)
        std::memmove(p_ + ch*HALF + 1, p_ + ch*HALF, (HALF-1) * sizeof(float));

    // Global decay and per-channel blur.
    for (int i = 0; i < 3*HALF; ++i) p_[i] *= 0.98f;
    for (int ch = 0; ch < 3; ++ch)
        gaussianFilter1dInPlace(p_ + ch*HALF, HALF, 0.2f);

    // New colour originates at the center (index 0 of each channel).
    p_[0*HALF] = r;
    p_[1*HALF] = g;
    p_[2*HALF] = b;
}

// ── Energy ────────────────────────────────────────────────────────────────────

void Visualizer::doEnergy(const float* mel) {
    float y[N];
    std::copy(mel, mel + N, y);
    gain_.update(y, N);
    for (int i = 0; i < N; ++i)
        y[i] = y[i] / std::max(gain_.value[i], 1e-6f) * static_cast<float>(HALF - 1);

    // Mean of each frequency third raised to 0.9 → bar length in pixels.
    constexpr int   T     = N / 3;
    constexpr float SCALE = 0.9f;
    auto bandMean = [&](int lo, int hi) -> int {
        float s = 0.0f;
        for (int i = lo; i < hi; ++i) s += std::pow(y[i], SCALE);
        return std::clamp(static_cast<int>(s / (hi - lo)), 0, HALF - 1);
    };
    int r = bandMean(0,   T);
    int g = bandMean(T,   2*T);
    int b = bandMean(2*T, N);

    // Hard 0/255 bar, then smoothed by p_filt_.
    float* pr = p_;
    float* pg = p_ + HALF;
    float* pb = p_ + 2*HALF;
    std::fill(pr, pr+r, 255.0f); std::fill(pr+r, pr+HALF, 0.0f);
    std::fill(pg, pg+g, 255.0f); std::fill(pg+g, pg+HALF, 0.0f);
    std::fill(pb, pb+b, 255.0f); std::fill(pb+b, pb+HALF, 0.0f);

    p_filt_.update(p_, 3*HALF);
    for (int i = 0; i < 3*HALF; ++i) p_[i] = std::round(p_filt_.value[i]);

    // Blur edges of each channel bar.
    for (int ch = 0; ch < 3; ++ch)
        gaussianFilter1dInPlace(p_ + ch*HALF, HALF, 4.0f);
}

// ── Spectrum ──────────────────────────────────────────────────────────────────

void Visualizer::doSpectrum(const float* mel, PixelFrame& out) {
    // Stretch 24 mel bins across the HALF-pixel half-strip.
    float y[HALF];
    linearInterp(mel, N, y, HALF);

    // R = mel energy minus slow common-mode baseline.
    common_mode_.update(y, HALF);
    float r_in[HALF];
    for (int i = 0; i < HALF; ++i) r_in[i] = y[i] - common_mode_.value[i];
    r_filt_.update(r_in, HALF);

    // G = absolute frame-to-frame difference (transient content).
    float diff[HALF];
    for (int i = 0; i < HALF; ++i) {
        diff[i]           = y[i] - prev_spectrum_[i];
        prev_spectrum_[i] = y[i];
    }

    // B = smoothed raw mel energy.
    b_filt_.update(y, HALF);

    // Write mirrored output directly — spectrum doesn't use p_.
    for (int i = 0; i < HALF; ++i) {
        uint8_t rv = clamp255(r_filt_.value[i]   * 255.0f);
        uint8_t gv = clamp255(std::abs(diff[i])  * 255.0f);
        uint8_t bv = clamp255(b_filt_.value[i]   * 255.0f);
        out[0][HALF-1-i] = rv;  out[0][HALF+i] = rv;
        out[1][HALF-1-i] = gv;  out[1][HALF+i] = gv;
        out[2][HALF-1-i] = bv;  out[2][HALF+i] = bv;
    }
}
