#include "audio.h"
#include "config.h"
#include <pthread.h>
#include <stdbool.h>
#include <stdio.h>
#include <threads.h>
#include <unistd.h>

void print_buffer(float buffer[MIC_RATE / FPS]) {
  // printf("First sample: %f, Last sample: %f", buffer[0], buffer[MIC_RATE /
  // FPS - 1]);
}

int main() {
  pthread_t thread_id;

  pthread_create(&thread_id, NULL, start_audio_stream, print_buffer);

  sleep(10);

  pthread_join(thread_id, NULL);
  return 0;
}
