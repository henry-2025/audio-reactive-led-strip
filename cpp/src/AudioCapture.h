#pragma once
#include "Config.h"
#include <portaudio.h>
#include <atomic>
#include <array>
#include <cstddef>

// Lock-free SPSC ring buffer for audio samples.
// The PortAudio callback (producer) writes; the main thread (consumer) reads.
// Capacity must be a power of 2.
class AudioRingBuffer {
public:
    static constexpr size_t CAPACITY = 8192; // must be power of 2

    size_t write(const float* data, size_t count);
    size_t read(float* data, size_t count);
    size_t available() const;
    void   clear();

private:
    std::array<float, CAPACITY> buf_{};
    std::atomic<size_t> write_pos_{0};
    std::atomic<size_t> read_pos_{0};
};

// Wraps a PortAudio input stream and exposes audio frames to the main thread.
// Converts int16 microphone samples to normalised float32 (range ±1).
class AudioCapture {
public:
    AudioCapture();
    ~AudioCapture();

    // Returns true if a full frame of SAMPLES_PER_FRAME samples is ready.
    // Blocks are never held — returns false if not enough data yet.
    bool readFrame(float* out, int n_samples = Config::SAMPLES_PER_FRAME);

    size_t overflowCount() const { return overflows_.load(); }

private:
    static int paCallback(const void* input, void* output,
                          unsigned long frame_count,
                          const PaStreamCallbackTimeInfo* time_info,
                          PaStreamCallbackFlags flags,
                          void* user_data);

    PaStream*          stream_   = nullptr;
    AudioRingBuffer    ring_;
    std::atomic<size_t> overflows_{0};
};
