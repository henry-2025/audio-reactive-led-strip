#include "audio.h"

#include "config.h"
#include <pulse/simple.h>
#include <stdbool.h>
#include <stdio.h>
#include <time.h>

#ifdef linux
pa_simple *create_audio_stream() {
  pa_simple *s;
  pa_sample_spec ss;
  ss.format = PA_SAMPLE_FLOAT32;
  ss.channels = 1;
  ss.rate = MIC_RATE;

  int error;

  s = pa_simple_new(NULL, "Reactive Desktop", PA_STREAM_RECORD, NULL,
                    "Record audio stream", &ss, NULL, NULL, &error);
  if (s == NULL) {
    printf("Could not create PA device: %d\n", error);
    exit(EXIT_FAILURE);
  }

  return s;
}

volatile bool should_close;

void *start_audio_stream(void *data) {
  pa_simple *audio_stream = create_audio_stream();
  float buffer[MIC_RATE / FPS];

  audio_thread_values_t *passed_values = (audio_thread_values_t *)data;

  int error;

  // Flush buffers before we start reading
  if (pa_simple_flush(audio_stream, &error) < 0) {
    printf("Could not flush PA device: %d\n", error);
    exit(EXIT_FAILURE);
  }
  int samples_read = 0;
  while (!should_close) {
    // mutate the buffer, sampling in system audio
    pthread_mutex_lock(passed_values->mutex);
    if (pa_simple_read(audio_stream, buffer, (MIC_RATE / FPS) * sizeof(float),
                       &error) < 0) {
      printf("Could not read PA buffer: %d\n", error);
      exit(EXIT_FAILURE);
    }
    pthread_mutex_unlock(passed_values->mutex);

    // callback on the samples (fft and put to opengl buffer)
    passed_values->callback(buffer);
  }

  pa_simple_free(audio_stream);

  return NULL;
}
#endif

#ifdef macos

void start_audio_stream(void callback(float *buffer)) {
  // TODO: implement
}

#endif

#ifdef windows

void start_audio_stream(void callback(float *buffer)) {
  // TODO: implement
}

#endif
