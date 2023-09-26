#ifndef LED_H
#define LED_H

#include "config.h"
#include <stdbool.h>

int get_esp_conn(char *dev_ip, int dev_port);

void _update_esp8266(char pixels[N_PIXELS][3], char prev_pixels[N_PIXELS][3],
                     bool gamma_correction, int client_fd);
#endif
