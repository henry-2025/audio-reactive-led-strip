#include "Config.h"
#include "AudioCapture.h"
#include "DSP.h"
#include <cstdio>
#include <chrono>
#include <thread>

int main() {
    AudioCapture audio;
    DSPPipeline  dsp;

    float frame[Config::SAMPLES_PER_FRAME];

    printf("Audio/DSP pipeline running. Press Ctrl-C to quit.\n");

    while (true) {
        if (!audio.readFrame(frame))
            continue;

        bool active = dsp.update(frame);
        if (!active) {
            printf("Volume below threshold\n");
            continue;
        }

        // Print the 24 mel bins as a simple ASCII bar chart
        const float* mel = dsp.mel();
        printf("\r");
        for (int i = 0; i < Config::N_FFT_BINS; ++i) {
            int bars = static_cast<int>(mel[i] * 20.0f);
            printf("%2d ", bars);
        }
        fflush(stdout);

        // Pace to ~60 FPS (visualizer will drive this loop properly later)
        std::this_thread::sleep_for(std::chrono::milliseconds(1000 / Config::FPS));
    }
}
