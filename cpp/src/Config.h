#pragma once

namespace Config {

// Audio
inline constexpr int   MIC_RATE            = 44100;
inline constexpr int   FPS                 = 60;
inline constexpr int   N_ROLLING_HISTORY   = 2;
inline constexpr float MIN_FREQUENCY       = 200.0f;
inline constexpr float MAX_FREQUENCY       = 12000.0f;
inline constexpr float MIN_VOLUME_THRESHOLD = 1e-7f;

// LED
inline constexpr int   N_PIXELS            = 134;
inline constexpr int   N_FFT_BINS          = 24;

// Derived audio constants
inline constexpr int SAMPLES_PER_FRAME    = MIC_RATE / FPS;           // 735
inline constexpr int ROLLING_WINDOW_SIZE  = SAMPLES_PER_FRAME * N_ROLLING_HISTORY; // 1470
inline constexpr int FFT_SIZE             = 2048; // next power of 2 >= 1470
// Only the first N//2 FFT bins are used, matching the Python slice [:N//2]
inline constexpr int FFT_BINS_USED        = ROLLING_WINDOW_SIZE / 2;  // 735

} // namespace Config
