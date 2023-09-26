#include "led.h"
#include <arpa/inet.h>
#include <netdb.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

int setup_server() {
  int server_fd, new_socket, status;
  struct sockaddr_in address;
  int opt = 1;
  int addrlen = sizeof(address);
  // Creating socket file descriptor
  if ((server_fd = socket(AF_INET, SOCK_DGRAM, 0)) < 0) {
    perror("socket failed");
    return -1;
  }

  // Forcefully attaching socket to the port 7777
  if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt,
                 sizeof(opt))) {
    perror("setsockopt");
    return -1;
  }
  address.sin_family = AF_INET;
  address.sin_addr.s_addr = INADDR_ANY;
  address.sin_port = htons(DEV_PORT);

  // Forcefully attaching socket to the port 7777
  if (bind(server_fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
    perror("bind failed");
    return -1;
  }

  return server_fd;
}

int main() {

  int server_fd = setup_server();
  if (server_fd < 0) {
    return server_fd;
  }

  int client_fd = get_esp_conn("127.0.0.1", 7777);

  char pixels[N_PIXELS][3], prev_pixels[N_PIXELS][3];
  memset(pixels, 0, 3 * sizeof(char) * N_PIXELS);
  memset(prev_pixels, 0, 3 * sizeof(char) * N_PIXELS);

  pixels[0][0] = 100;
  pixels[0][1] = 10;
  pixels[0][2] = 1;

  _update_esp8266(pixels, prev_pixels, true, client_fd);

  char buffer[100];
  int valread = read(server_fd, buffer, 100);
  for (int i = 0; i < 4; i++) {
    printf("%d, ", buffer[i]);
  }

  close(client_fd);

  shutdown(server_fd, SHUT_RDWR);
  return 0;
}
