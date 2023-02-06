#include <NeoPixelBus.h>

// Set to the number of LEDs in your LED strip
#define NUM_LEDS 100
#define BUFFER_LEN (NUM_LEDS * 3)
// Toggles FPS output (1 = print FPS over serial, 0 = disable output)
#define PRINT_FPS 0
#define BUFFER_DEBUG 1

// NeoPixelBus settings
const uint8_t PixelPin = 2; // make sure to set this to the correct pin, ignored for Esp8266(set to 3 by default for DMA)

// buffer for serial reads
uint8_t buffer[BUFFER_LEN];

uint8_t N = 0;

NeoPixelBus<NeoGrbFeature, Neo800KbpsMethod> ledstrip(NUM_LEDS, PixelPin);

void setup()
{
    Serial.begin(115200);
    ledstrip.Begin(); // Begin output
    ledstrip.SetPixelColor(1, RgbColor(0, 0, 100));
    ledstrip.Show(); // Clear the strip for use
    Serial.println("Controller initialized");
}

#if PRINT_FPS
uint16_t fpsCounter = 0;
uint32_t secondTimer = 0;
#endif

void loop()
{
    /*
    Read raw bytes over serial and assign color strip to the unpacked bytes
    */
    if (Serial.available())
    {
        Serial.readBytes(buffer, BUFFER_LEN);
#if BUFFER_DEBUG
        for (int i = 0; i < BUFFER_LEN; i++)
        {
            Serial.print(buffer[i]);
            Serial.print(", ");
        }
        Serial.println("");
#endif
        for (int i = 0; i < BUFFER_LEN; i++)
        {
            RgbColor pixel((uint8_t)buffer[3 * i], (uint8_t)buffer[3 * i + 2], (uint8_t)buffer[3 * i + 1]); // color. RgbColor accepts bytes in rbg order btw
            ledstrip.SetPixelColor(i, pixel);                                                               // i is the pixel number
        }
        ledstrip.Show();
#if PRINT_FPS
        fpsCounter++;
#endif
    }
#if PRINT_FPS
    if (millis() - secondTimer >= 1000U)
    {
        secondTimer = millis();
        Serial.print("FPS: ");
        Serial.println(fpsCounter, DEC);
        fpsCounter = 0;
    }
#endif
}
