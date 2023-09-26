#include "config.h"
#include "dsp.h"
#include <assert.h>
#include <math.h>

int main() {
  float mel_y[NUM_BANDS][N_FFT_BANDS];
  float mel_x[N_FFT_BANDS];
  int samples;

  struct config cfg = {.freq_min = 10,
                       .freq_max = 10000,
                       .sample_rate = 60,
                       .mic_rate = 60,
                       .n_rolling_history = 10,
                       .fps = 60};

  create_mel_bank(10, mel_y, mel_x, &samples, cfg);

  float epsilon = 1e-3;
  for (int i = 0; i < NUM_BANDS; i++) {
      for (int j = 0; j < N_FFT_BANDS; j++) {
          printf("%f ", mel_y[i][j]);
      }
      printf("\n");
  }
  if (fabs(mel_y[0][0]) < epsilon) {
    return 1;
  } else {
      return 0;
  }
}
