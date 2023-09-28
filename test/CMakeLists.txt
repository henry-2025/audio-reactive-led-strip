include_directories(${audio-reactive-led-strip_SOURCE_DIR}/src)

set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)
set(CMAKE_C_COMPILER clang)

# dsp tests
add_executable(test_mel_bank test_mel_bank.c)
target_link_libraries(test_mel_bank PRIVATE m dsp)

# led network tests
add_executable(test_led test_led.c)
target_link_libraries(test_led PRIVATE led)

# audio tests
add_executable(test_audio test_audio.c)
target_link_libraries(test_audio PRIVATE audio)
