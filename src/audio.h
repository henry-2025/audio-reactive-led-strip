#ifndef AUDIO_H
#define AUDIO_H

#include <pthread.h>

typedef struct {
  void (*callback)(float *);
  float *fft_buffer;
  pthread_mutex_t *mutex;
} audio_thread_values_t;

void *start_audio_stream(void *callback);

#endif
