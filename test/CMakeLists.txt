include_directories(${audio-reactive-led-strip_SOURCE_DIR}/src)

set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)
set(CMAKE_C_COMPILER clang)
find_package(PkgConfig REQUIRED)
pkg_check_modules(FFTW REQUIRED IMPORTED_TARGET fftw3)

# dsp tests
add_executable(test_mel_bank test_mel_bank.c)
target_link_libraries(test_mel_bank PRIVATE m dsp)
add_executable(test_exp_filter test_exp_filter.c)
target_link_libraries(test_exp_filter PRIVATE dsp)
add_executable(test_fft test_fft.c)
target_link_libraries(test_fft PRIVATE dsp PkgConfig::FFTW)


# led network tests
add_executable(test_led test_led.c)
target_link_libraries(test_led PRIVATE led)

# audio tests
add_executable(test_audio test_audio.c)
target_link_libraries(test_audio PRIVATE audio)

