cmake_minimum_required(VERSION 3.10.0)
project(audio-reactive-led-strip LANGUAGES CXX)

set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)

find_package(PkgConfig REQUIRED)
pkg_check_modules(GTK REQUIRED IMPORTED_TARGET gtk4)
pkg_check_modules(GLEW REQUIRED IMPORTED_TARGET glew)
pkg_check_modules(GLM REQUIRED IMPORTED_TARGET glm)
find_package(OpenGL)

add_executable(reactive main.cpp)
target_link_libraries(reactive PRIVATE PkgConfig::GTK PkgConfig::GLEW PkgConfig::GLM ${OPENGL_LIBRARIES})
include_directories(${GTK_INCLUDE_DIRS} ${GLEW_INCLUDE_DIRS} ${GLM_INCLUDE_DIRS})
