#ifndef CONFIG_H
#define CONFIG_H

#include <stdlib.h>

// compile and runtime configuration

#define NUM_BANDS 10
#define N_FFT_BANDS 24
#define N_FFT_BINS 24
#define N_PIXELS 255 // TODO: this should be something that we define in the config
#define DEV_IP "192.168.0.150"
#define DEV_PORT 7777

struct config {
  uint freq_min;
  uint freq_max;
  uint sample_rate;

  uint mic_rate;
  uint n_rolling_history;

  uint fps;
};
#endif
