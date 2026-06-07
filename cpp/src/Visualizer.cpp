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
        case Effect::Strobe:   doStrobe(mel);         mirrorOut(out); break;
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

    // ── Hue rotation ──────────────────────────────────────────────────────────
    // Sum mel bins (normalised by N so the metric stays in [0,1]).
    // Feed through an ExpFilter for inertia, then accumulate into an angle.
    // Result: the colour palette rotates faster during loud/energetic passages
    // and coasts slowly in quieter moments.
    float energy_sum = 0.0f;
    for (int i = 0; i < N; ++i) energy_sum += mel[i];
    hue_energy_.update(energy_sum / N);

    // Map filtered energy directly to a rotation angle in [0, MAX_ANGLE].
    // High energy → maximum colour rotation; silence → identity (no rotation).
    // 2π/3 (120°) is the natural unit for RGB: at max it gives a full
    // R→G→B→R channel permutation.
    constexpr float MAX_ANGLE = 2.0f * static_cast<float>(M_PI);
    hue_angle_ = std::clamp(hue_energy_.value, 0.0f, 1.0f) * MAX_ANGLE;

    // Hue rotation matrix: rotates around the (1,1,1) luminance axis (Rodrigues).
    // At θ=0 → identity; at θ=2π/3 → R→G→B→R cyclic permutation.
    const float c  = std::cos(hue_angle_);
    const float s  = std::sin(hue_angle_);
    const float k  = (1.0f - c) / 3.0f;
    const float sq = s / std::sqrt(3.0f);
    //          [ c+k    k-sq   k+sq ]
    //  R(θ) =  [ k+sq   c+k    k-sq ]
    //          [ k-sq   k+sq   c+k  ]

    // Apply rotation and write mirrored output — spectrum doesn't use p_.
    for (int i = 0; i < HALF; ++i) {
        float r = r_filt_.value[i];
        float g = std::abs(diff[i]);
        float b = b_filt_.value[i];

        uint8_t rv = clamp255(((c+k)*r + (k-sq)*g + (k+sq)*b) * 255.0f);
        uint8_t gv = clamp255(((k+sq)*r + (c+k)*g + (k-sq)*b) * 255.0f);
        uint8_t bv = clamp255(((k-sq)*r + (k+sq)*g + (c+k)*b) * 255.0f);

        out[0][HALF-1-i] = rv;  out[0][HALF+i] = rv;
        out[1][HALF-1-i] = gv;  out[1][HALF+i] = gv;
        out[2][HALF-1-i] = bv;  out[2][HALF+i] = bv;
    }
}

// ── Strobe ────────────────────────────────────────────────────────────────────

void Visualizer::doStrobe(const float* mel) {
    // Trigger threshold: envelope must exceed this to fire a flash.
    // Rearm threshold: decay must fall below this before the next flash can trigger.
    constexpr float STROBE_THRESHOLD = 0.3f;
    constexpr float STROBE_REARM     = 20.0f;  // brightness units (0-255)
    // Per-frame multiplicative decay applied while in the decay state.
    // ~0.85 at 60 fps gives roughly a 100 ms tail.
    constexpr float DECAY_RATE       = 0.85f;

    // Always update the envelope so it tracks music even while decaying.
    float energy = 0.0f;
    for (int i = 0; i < N; ++i) energy += mel[i];
    strobe_env_.update(energy / N);

    if (strobe_decaying_) {
        // Independent exponential decay — ignores FFT energy until rearmed.
        strobe_brightness_ *= DECAY_RATE;
        if (strobe_brightness_ < STROBE_REARM)
            strobe_decaying_ = false;
    } else if (strobe_env_.value >= STROBE_THRESHOLD) {
        // New flash: snap to full brightness and enter decay state.
        strobe_brightness_ = 255.0f;
        strobe_decaying_   = true;
    }
    // else: rearmed but below threshold — strip stays dark.

    std::fill(p_,          p_ + HALF,   strobe_brightness_);
    std::fill(p_ + HALF,   p_ + 2*HALF, strobe_brightness_);
    std::fill(p_ + 2*HALF, p_ + 3*HALF, strobe_brightness_);
}
