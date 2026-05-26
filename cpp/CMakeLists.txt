cmake_minimum_required(VERSION 3.20)
project(AudioReactiveLED LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ── Dependencies ──────────────────────────────────────────────────────────────

find_package(PkgConfig REQUIRED)

# PortAudio
pkg_check_modules(PORTAUDIO REQUIRED portaudio-2.0)

# FFTW3 (float precision: libfftw3f)
pkg_check_modules(FFTW3F REQUIRED fftw3f)

# ── Sources ───────────────────────────────────────────────────────────────────

set(CORE_SOURCES
    src/AudioCapture.cpp
    src/MelFilterbank.cpp
    src/DSP.cpp
)

# ── Targets ───────────────────────────────────────────────────────────────────

add_executable(led_visualizer src/main.cpp ${CORE_SOURCES})

target_include_directories(led_visualizer PRIVATE
    src/
    ${PORTAUDIO_INCLUDE_DIRS}
    ${FFTW3F_INCLUDE_DIRS}
)

target_link_directories(led_visualizer PRIVATE
    ${PORTAUDIO_LIBRARY_DIRS}
    ${FFTW3F_LIBRARY_DIRS}
)

target_link_libraries(led_visualizer PRIVATE
    ${PORTAUDIO_LIBRARIES}
    ${FFTW3F_LIBRARIES}
    "-framework CoreAudio"
    "-framework AudioToolbox"
    "-framework AudioUnit"
    "-framework CoreFoundation"
    "-framework CoreServices"
)

# Suppress duplicate-library warning from pkg-config double-adding fftw3f
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    target_link_options(led_visualizer PRIVATE -Wl,-no_warn_duplicate_libraries)
endif()

target_compile_options(led_visualizer PRIVATE
    ${PORTAUDIO_CFLAGS_OTHER}
    ${FFTW3F_CFLAGS_OTHER}
    -Wall -Wextra -O2
)
