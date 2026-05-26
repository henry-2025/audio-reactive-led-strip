#pragma once
#include "Visualizer.h"   // PixelFrame
#include <cstdint>
#include <string>

// ── Base class ────────────────────────────────────────────────────────────────

class LEDOutput {
public:
    virtual ~LEDOutput() = default;

    // Send one full frame to the LED strip.
    virtual void send(const PixelFrame& pixels) = 0;

    // Load a 256-entry gamma LUT from a NumPy .npy file (uint8 dtype).
    // Gamma is applied inside send() when enabled.
    bool loadGammaTable(const std::string& path);
    void setGammaEnabled(bool on) { gamma_enabled_ = on; }

protected:
    bool    gamma_enabled_ = false;
    uint8_t gamma_[256];

    uint8_t applyGamma(uint8_t v) const {
        return gamma_enabled_ ? gamma_[v] : v;
    }
};

// ── Null output (no device) ───────────────────────────────────────────────────
//
// Accepts frames and discards them. Useful for running the DSP/visualizer
// pipeline without any hardware attached.

class NullOutput : public LEDOutput {
public:
    void send(const PixelFrame& /*pixels*/) override {}
};

// ── Raspberry Pi Pico (serial) ────────────────────────────────────────────────
//
// Packet format: [0xFF, 0x00, N_PIXELS] + [R0, G0, B0, R1, G1, B1, ...]
// Baud rate: 460800

class PicoOutput : public LEDOutput {
public:
    explicit PicoOutput(const std::string& port, int baud_rate = 460800);
    ~PicoOutput() override;

    void send(const PixelFrame& pixels) override;

private:
    int fd_ = -1;
};

// ── ESP8266 (UDP) ─────────────────────────────────────────────────────────────
//
// Packet format: [index, R, G, B] per changed pixel, up to 126 pixels/packet.

class ESP8266Output : public LEDOutput {
public:
    ESP8266Output(const std::string& ip, int port = 7777);
    ~ESP8266Output() override;

    void send(const PixelFrame& pixels) override;

private:
    static constexpr int MAX_PER_PACKET = 126;

    int  sock_ = -1;
    // sockaddr_in stored as opaque bytes to avoid pulling in net headers here.
    uint8_t addr_[16]{};

    // Tracks the last-sent frame to skip unchanged pixels.
    PixelFrame prev_{};
};
