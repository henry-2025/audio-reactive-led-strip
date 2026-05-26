#include "AudioCapture.h"
#include <stdexcept>
#include <cstring>
#include <algorithm>

// ── AudioRingBuffer ───────────────────────────────────────────────────────────

size_t AudioRingBuffer::write(const float* data, size_t count) {
    size_t wp = write_pos_.load(std::memory_order_relaxed);
    size_t rp = read_pos_.load(std::memory_order_acquire);
    size_t space = CAPACITY - (wp - rp);
    count = std::min(count, space);
    for (size_t i = 0; i < count; ++i)
        buf_[(wp + i) & (CAPACITY - 1)] = data[i];
    write_pos_.store(wp + count, std::memory_order_release);
    return count;
}

size_t AudioRingBuffer::read(float* data, size_t count) {
    size_t rp = read_pos_.load(std::memory_order_relaxed);
    size_t wp = write_pos_.load(std::memory_order_acquire);
    size_t avail = wp - rp;
    count = std::min(count, avail);
    for (size_t i = 0; i < count; ++i)
        data[i] = buf_[(rp + i) & (CAPACITY - 1)];
    read_pos_.store(rp + count, std::memory_order_release);
    return count;
}

size_t AudioRingBuffer::available() const {
    size_t wp = write_pos_.load(std::memory_order_acquire);
    size_t rp = read_pos_.load(std::memory_order_relaxed);
    return wp - rp;
}

void AudioRingBuffer::clear() {
    read_pos_.store(write_pos_.load(std::memory_order_acquire),
                    std::memory_order_release);
}

// ── AudioCapture ─────────────────────────────────────────────────────────────

AudioCapture::AudioCapture() {
    PaError err = Pa_Initialize();
    if (err != paNoError)
        throw std::runtime_error(Pa_GetErrorText(err));

    PaStreamParameters params{};
    params.device           = Pa_GetDefaultInputDevice();
    params.channelCount     = 1;
    params.sampleFormat     = paInt16;
    params.suggestedLatency = Pa_GetDeviceInfo(params.device)->defaultLowInputLatency;

    err = Pa_OpenStream(&stream_,
                        &params,
                        nullptr,                      // no output
                        Config::MIC_RATE,
                        Config::SAMPLES_PER_FRAME,    // frames per callback buffer
                        paClipOff,
                        &AudioCapture::paCallback,
                        this);
    if (err != paNoError) {
        Pa_Terminate();
        throw std::runtime_error(Pa_GetErrorText(err));
    }

    err = Pa_StartStream(stream_);
    if (err != paNoError) {
        Pa_CloseStream(stream_);
        Pa_Terminate();
        throw std::runtime_error(Pa_GetErrorText(err));
    }
}

AudioCapture::~AudioCapture() {
    if (stream_) {
        Pa_StopStream(stream_);
        Pa_CloseStream(stream_);
    }
    Pa_Terminate();
}

bool AudioCapture::readFrame(float* out, int n_samples) {
    if (static_cast<int>(ring_.available()) < n_samples)
        return false;
    ring_.read(out, static_cast<size_t>(n_samples));
    return true;
}

int AudioCapture::paCallback(const void* input, void* /*output*/,
                             unsigned long frame_count,
                             const PaStreamCallbackTimeInfo* /*time_info*/,
                             PaStreamCallbackFlags /*flags*/,
                             void* user_data)
{
    auto* self = static_cast<AudioCapture*>(user_data);
    const auto* samples = static_cast<const int16_t*>(input);

    // Convert int16 to normalised float32 and push to ring buffer
    static thread_local float tmp[Config::SAMPLES_PER_FRAME * 4];
    size_t n = std::min(static_cast<size_t>(frame_count), sizeof(tmp)/sizeof(tmp[0]));
    for (size_t i = 0; i < n; ++i)
        tmp[i] = samples[i] / 32768.0f;

    size_t written = self->ring_.write(tmp, n);
    if (written < n)
        self->overflows_.fetch_add(1, std::memory_order_relaxed);

    return paContinue;
}
