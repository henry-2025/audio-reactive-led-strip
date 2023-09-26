cmake_minimum_required(VERSION 3.10.0)
project(audio-reactive-led-strip LANGUAGES C)


find_package(PkgConfig REQUIRED)
pkg_check_modules(GTK REQUIRED IMPORTED_TARGET gtk4)
pkg_check_modules(FFTW REQUIRED IMPORTED_TARGET fftw3f)
find_package(GLEW REQUIRED)
set(OpenGL_GL_PREFERENCE GLVND)
find_package(OpenGL REQUIRED)

set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)
set(CMAKE_C_COMPILER clang)
include_directories(${GTK_INCLUDE_DIRS} ${GLEW_INCLUDE_DIRS} ${FFTW_INCLUDE_DIRS})

# main executable
add_executable(reactive main.c)
target_link_libraries(reactive PRIVATE ${GLEW_LIBRARIES} ${OPENGL_LIBRARIES} m PkgConfig::GTK PkgConfig::FFTW)

enable_testing()
# dsp tests
add_executable(test_mel_bank test_mel_bank.c dsp.c dsp.h config.h)
target_link_libraries(test_mel_bank PRIVATE m PkgConfig::FFTW)
add_test(NAME TestMelBank COMMAND test_mel_bank)

# led network tests
add_executable(test_led test_led.c led.c led.h config.h gamma_table.c gamma_table.h)
add_test(NAME TestLED COMMAND test_led)
