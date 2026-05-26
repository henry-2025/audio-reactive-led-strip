#include "LEDOutput.h"
#include <stdexcept>
#include <cstring>
#include <fstream>

// POSIX serial
#include <fcntl.h>
#include <termios.h>
#include <unistd.h>

// BSD sockets
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

// ── LEDOutput base ────────────────────────────────────────────────────────────

// Minimal .npy v1/v2 reader for a flat uint8 array.
bool LEDOutput::loadGammaTable(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) return false;

    char magic[6]{};
    f.read(magic, 6);
    if (std::memcmp(magic, "\x93NUMPY", 6) != 0) return false;

    uint8_t ver[2]{};
    f.read(reinterpret_cast<char*>(ver), 2);

    // Header length: 2 bytes for v1.x, 4 bytes for v2.x
    uint32_t header_len = 0;
    if (ver[0] == 1) {
        uint16_t hl = 0;
        f.read(reinterpret_cast<char*>(&hl), 2);
        header_len = hl;
    } else {
        f.read(reinterpret_cast<char*>(&header_len), 4);
    }
    f.seekg(header_len, std::ios::cur);

    f.read(reinterpret_cast<char*>(gamma_), 256);
    return f.good();
}

// ── PicoOutput ────────────────────────────────────────────────────────────────

static speed_t baudToSpeed(int baud) {
    switch (baud) {
        case 9600:   return B9600;
        case 19200:  return B19200;
        case 38400:  return B38400;
        case 57600:  return B57600;
        case 115200: return B115200;
        // 460800 is non-POSIX but supported on Linux and macOS.
#ifdef B460800
        case 460800: return B460800;
#endif
        default:     return B115200;
    }
}

PicoOutput::PicoOutput(const std::string& port, int baud_rate) {
    fd_ = open(port.c_str(), O_RDWR | O_NOCTTY | O_SYNC);
    if (fd_ < 0)
        throw std::runtime_error("PicoOutput: cannot open " + port);

    termios tty{};
    if (tcgetattr(fd_, &tty) != 0)
        throw std::runtime_error("PicoOutput: tcgetattr failed");

    speed_t speed = baudToSpeed(baud_rate);
    cfsetispeed(&tty, speed);
    cfsetospeed(&tty, speed);

    cfmakeraw(&tty);
    tty.c_cc[VMIN]  = 0;
    tty.c_cc[VTIME] = 0;

    if (tcsetattr(fd_, TCSANOW, &tty) != 0)
        throw std::runtime_error("PicoOutput: tcsetattr failed");
}

PicoOutput::~PicoOutput() {
    if (fd_ >= 0) close(fd_);
}

void PicoOutput::send(const PixelFrame& pixels) {
    constexpr int N = Config::N_PIXELS;
    // 3-byte header + N*3 bytes of interleaved RGB.
    uint8_t buf[3 + N * 3];
    buf[0] = 0xFF;
    buf[1] = 0x00;
    buf[2] = static_cast<uint8_t>(N);
    for (int i = 0; i < N; ++i) {
        buf[3 + i*3 + 0] = applyGamma(pixels[0][i]);
        buf[3 + i*3 + 1] = applyGamma(pixels[1][i]);
        buf[3 + i*3 + 2] = applyGamma(pixels[2][i]);
    }
    ::write(fd_, buf, sizeof(buf));
}

// ── ESP8266Output ─────────────────────────────────────────────────────────────

ESP8266Output::ESP8266Output(const std::string& ip, int port) {
    sock_ = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock_ < 0)
        throw std::runtime_error("ESP8266Output: socket() failed");

    auto* addr = reinterpret_cast<sockaddr_in*>(addr_);
    std::memset(addr, 0, sizeof(sockaddr_in));
    addr->sin_family      = AF_INET;
    addr->sin_port        = htons(static_cast<uint16_t>(port));
    addr->sin_addr.s_addr = inet_addr(ip.c_str());

    // Initialise prev_ to a value that differs from any real frame so the
    // first send transmits all pixels.
    for (auto& ch : prev_)
        ch.fill(253);
}

ESP8266Output::~ESP8266Output() {
    if (sock_ >= 0) close(sock_);
}

void ESP8266Output::send(const PixelFrame& pixels) {
    constexpr int N = Config::N_PIXELS;
    const auto* addr = reinterpret_cast<const sockaddr*>(addr_);

    // Collect changed-pixel indices then chunk into UDP packets.
    // Packet layout: [i, R, G, B] per pixel.
    uint8_t buf[MAX_PER_PACKET * 4];
    int     buf_len = 0;

    auto flush = [&]() {
        if (buf_len > 0) {
            sendto(sock_, buf, static_cast<size_t>(buf_len), 0, addr, sizeof(sockaddr_in));
            buf_len = 0;
        }
    };

    for (int i = 0; i < N; ++i) {
        uint8_t r = applyGamma(pixels[0][i]);
        uint8_t g = applyGamma(pixels[1][i]);
        uint8_t b = applyGamma(pixels[2][i]);

        if (r == prev_[0][i] && g == prev_[1][i] && b == prev_[2][i])
            continue;

        if (buf_len == MAX_PER_PACKET * 4)
            flush();

        buf[buf_len++] = static_cast<uint8_t>(i);
        buf[buf_len++] = r;
        buf[buf_len++] = g;
        buf[buf_len++] = b;
    }
    flush();

    prev_ = pixels;
}
