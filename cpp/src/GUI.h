#pragma once
#include "Config.h"
#include "ExpFilter.h"
#include "Visualizer.h"

// Owns the SDL2 window, OpenGL context, and all ImGui/ImPlot state.
//
// Typical usage per frame:
//   if (!gui.pollEvents()) break;
//   gui.beginFrame();
//   bool freq_changed = gui.draw(mel, viz, pixels, fps);
//   if (freq_changed) dsp.setFrequencyRange(gui.minHz(), gui.maxHz());
//   gui.endFrame();
class GUI {
public:
    GUI();
    ~GUI();

    // Pump SDL events. Returns false when the window is closed.
    bool pollEvents();

    void beginFrame();

    // Render controls and plots. Returns true if the frequency range slider moved.
    bool draw(const float* mel, Visualizer& viz, const PixelFrame& pixels, float fps);

    void endFrame();

    // Current frequency range in Hz (read after draw() returns true).
    float minHz() const;
    float maxHz() const;

private:
    // SDL/GL handles stored as void* to keep SDL2 out of this header.
    void* window_  = nullptr;
    void* gl_ctx_  = nullptr;

    // Frequency slider handles in normalised [0,1] space.
    // Maps non-linearly: freq = t^2 * (MIC_RATE/2), matching the Python tick slider.
    float freq_lo_ = std::sqrt(Config::MIN_FREQUENCY / (Config::MIC_RATE / 2.0f));
    float freq_hi_ = std::sqrt(Config::MAX_FREQUENCY / (Config::MIC_RATE / 2.0f));

    // Which handle is being dragged (set once at drag start).
    bool drag_lo_ = false;

    // Display-only smoothing for the mel plot (matches Python fft_plot_filter).
    ExpFilterArray fft_plot_filter_{Config::N_FFT_BINS, 0.1f, 0.5f, 0.99f};

    // Custom double-handle frequency slider. Returns true when a handle moves.
    bool freqRangeSlider();
};
