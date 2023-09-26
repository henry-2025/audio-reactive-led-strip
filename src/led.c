#include "config.h"
#include "gamma_table.h"
#include <arpa/inet.h>
#include <netdb.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>

int get_esp_conn(char *dev_ip, int dev_port) {
  int client_fd, status;

  // first, load up address structs with getaddrinfo():
  struct sockaddr_in dev_addr;
  if ((client_fd = socket(AF_INET, SOCK_DGRAM, 0)) < 0) {
    perror("Socket creation failed\n");
    exit(EXIT_FAILURE);
  }

  dev_addr.sin_family = AF_INET;
  dev_addr.sin_port = htons(dev_port);

  if (inet_pton(AF_INET, dev_ip, &dev_addr.sin_addr) <= 0) {
    perror("Unvalid address/ Address not supported \n");
    exit(EXIT_FAILURE);
  }

  // don't technically need to connect, but we do so for convenience because we
  // only have one target
  if ((status = connect(client_fd, (struct sockaddr *)&dev_addr,
                        sizeof(dev_addr))) < 0) {
    perror("Connection to device failed");
    exit(EXIT_FAILURE);
  }
  return client_fd;
}

void _update_esp8266(char pixels[N_PIXELS][3],
                     char prev_pixels[N_PIXELS][3], bool gamma_correction,
                     int client_fd) {
  // do gamma correction
  char pixels_gamma[N_PIXELS][3];
  if (gamma_correction) {
    for (int i = 0; i < N_PIXELS; i++) {
      for (int j = 0; j < 3; j++) {
        pixels_gamma[i][j] = gamma_table[pixels[i][j]];
      }
    }
    pixels = pixels_gamma;
  }

  /** get stack of pixels change indices
   * Packet encoding scheme is:
   * |i|r|g|b| where
   *
   *    i (0 to 255): Index of LED to change (zero-based)
   *    r (0 to 255): Red value of LED
   *    g (0 to 255): Green value of LED
   *    b (0 to 255): Blue value of LED
   */

  char updated_pixels[N_PIXELS][4];
  int k = 0;
  for (int i = 0; i < N_PIXELS; i++) {
    for (int j = 0; j < 3; j++) {
      if (pixels[i][j] != prev_pixels[i][j]) {
        updated_pixels[k][0] = i;
        for (int l = 1; l < 4; l++) {
          updated_pixels[k][l - 1] = pixels[i][l - 1];
        }
        k++;
        break;
      }
    }
  }

  send(client_fd, updated_pixels, k * 4 * sizeof(char), 0);
}
