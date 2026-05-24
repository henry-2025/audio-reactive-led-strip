/*
 * Raspberry Pi Pico WS2812 serial slave controller
 *
 * Receives RGB frames over USB CDC serial from the Python DSP host and drives
 * a WS2812B strip via FastLED (uses PIO, so no interrupt conflicts with USB).
 *
 * Board: Raspberry Pi Pico (Earle Philhower arduino-pico core)
 * Library: FastLED >= 3.6
 *
 * Protocol (matches _update_pico() in led.py):
 *   [0xFF] [0x00] [n: uint8] [R0 G0 B0 R1 G1 B1 ... Rn-1 Gn-1 Bn-1]
 *
 * n must equal NUM_LEDS; frames with a mismatched count are discarded.
 * Both sides must agree on NUM_LEDS (set config.N_PIXELS in config.py to match).
 */

#include <FastLED.h>

#define NUM_LEDS   134
#define DATA_PIN   0    // GP0 — connect to output of 74AHCT125 level shifter
#define PRINT_FPS  1

CRGB leds[NUM_LEDS];

static uint8_t frameBuf[NUM_LEDS * 3];

#if PRINT_FPS
static uint16_t fpsCounter = 0;
static uint32_t secondTimer = 0;
#endif

void setup() {
    Serial.begin();  // USB CDC — baud rate argument is ignored on Pico
    FastLED.addLeds<WS2812B, DATA_PIN, GRB>(leds, NUM_LEDS);
    FastLED.clear();
    FastLED.show();
}

void loop() {
    if (!Serial.available()) return;
    if (Serial.read() != 0xFF) return;

    while (!Serial.available());
    if (Serial.read() != 0x00) return;

    while (!Serial.available());
    uint8_t n = Serial.read();
    if (n != NUM_LEDS) return;

    // Block until the full frame arrives
    uint16_t remaining = (uint16_t)n * 3;
    uint16_t received = 0;
    while (received < remaining) {
        if (Serial.available()) {
            frameBuf[received++] = Serial.read();
        }
    }

    for (int i = 0; i < n; i++) {
        leds[i] = CRGB(frameBuf[i * 3], frameBuf[i * 3 + 1], frameBuf[i * 3 + 2]);
    }
    FastLED.show();

#if PRINT_FPS
    fpsCounter++;
    if (millis() - secondTimer >= 1000U) {
        secondTimer = millis();
        Serial.printf("FPS: %d\n", fpsCounter);
        fpsCounter = 0;
    }
#endif
}
