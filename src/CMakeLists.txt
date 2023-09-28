find_package(PkgConfig REQUIRED)
pkg_check_modules(GTK REQUIRED IMPORTED_TARGET gtk4)
pkg_check_modules(FFTW REQUIRED IMPORTED_TARGET fftw3f)
pkg_check_modules(PULSE REQUIRED IMPORTED_TARGET libpulse-simple)
find_package(GLEW REQUIRED)
set(OpenGL_GL_PREFERENCE GLVND)
find_package(OpenGL REQUIRED)


include_directories(${GTK_INCLUDE_DIRS} ${GLEW_INCLUDE_DIRS} ${FFTW_INCLUDE_DIRS} ${PULSE_INCLUDE_DIRS})
link_libraries(${GLEW_LIBRARIES} ${OPENGL_LIBRARIES} m PkgConfig::GTK PkgConfig::FFTW PkgConfig::PULSE)

set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)
set(CMAKE_C_COMPILER clang)

# main executable
file(GLOB AllSources *.c)
file(GLOB AllHeaders *.h)
add_executable(reactive ${AllSources} ${AllHeaders})

add_library(dsp dsp.c dsp.h)
add_library(audio audio.c audio.h)
add_library(led led.c led.h gamma_table.c gamma_table.h)
