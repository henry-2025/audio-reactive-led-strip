cmake_minimum_required(VERSION 3.10.0)
project(audio-reactive-led-strip LANGUAGES C)


add_subdirectory(src)
add_subdirectory(test)

set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)
set(CMAKE_C_COMPILER clang)

enable_testing()
add_test(NAME TestMelBank COMMAND test_mel_bank)
add_test(NAME TestLED COMMAND test_led)
add_test(NAME TestAudio COMMAND test_audio)

