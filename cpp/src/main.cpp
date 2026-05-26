#include "Config.h"
#include "AudioCapture.h"
#include "DSP.h"
#include "Visualizer.h"
#include "LEDOutput.h"
#include "GUI.h"

#include <chrono>
#include <cstdio>
#include <cstring>
#include <memory>
#include <string>

// Usage:
//   led_visualizer                           # no hardware (NullOutput)
//   led_visualizer pico  /dev/tty.usbmodem  # Raspberry Pi Pico via serial
//   led_visualizer esp   192.168.0.150       # ESP8266 via UDP

static std::unique_ptr<LEDOutput> makeLEDOutput(int argc, char** argv) {
    if (argc >= 2 && std::strcmp(argv[1], "pico") == 0) {
        std::string port = (argc >= 3) ? argv[2] : "/dev/tty.usbmodem1101";
        std::printf("LED: Pico on %s\n", port.c_str());
        return std::make_unique<PicoOutput>(port);
    }
    if (argc >= 2 && std::strcmp(argv[1], "esp") == 0) {
        std::string ip = (argc >= 3) ? argv[2] : "192.168.0.150";
        std::printf("LED: ESP8266 at %s\n", ip.c_str());
        return std::make_unique<ESP8266Output>(ip);
    }
    std::printf("LED: no device (NullOutput)\n");
    return std::make_unique<NullOutput>();
}

int main(int argc, char** argv) {
    AudioCapture audio;
    DSPPipeline  dsp;
    Visualizer   viz;
    viz.setEffect(Visualizer::Effect::Spectrum);

    auto led = makeLEDOutput(argc, argv);
    GUI  gui;

    float      frame[Config::SAMPLES_PER_FRAME];
    PixelFrame pixels{};

    // Simple exponential moving average for FPS display
    using Clock = std::chrono::steady_clock;
    auto   t_prev = Clock::now();
    float  fps    = static_cast<float>(Config::FPS);

    while (true) {
        // ── Events ────────────────────────────────────────────────────────────
        if (!gui.pollEvents()) break;

        // ── Audio / DSP / Visualizer ──────────────────────────────────────────
        if (audio.readFrame(frame)) {
            if (!dsp.update(frame))
                for (auto& ch : pixels) ch.fill(0);
            else
                viz.process(dsp.mel(), pixels);

            led->send(pixels);
        }

        // ── FPS ───────────────────────────────────────────────────────────────
        auto  t_now = Clock::now();
        float dt    = std::chrono::duration<float>(t_now - t_prev).count();
        t_prev = t_now;
        if (dt > 0.0f) fps = fps * 0.95f + (1.0f / dt) * 0.05f;

        // ── GUI ───────────────────────────────────────────────────────────────
        gui.beginFrame();
        bool freq_changed = gui.draw(dsp.mel(), viz, pixels, fps);
        gui.endFrame();

        if (freq_changed)
            dsp.setFrequencyRange(gui.minHz(), gui.maxHz());
    }
}
