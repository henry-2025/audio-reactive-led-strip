cmake_minimum_required(VERSION 3.20)
project(AudioReactiveLED LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ── Dependencies ──────────────────────────────────────────────────────────────

find_package(PkgConfig REQUIRED)

pkg_check_modules(PORTAUDIO REQUIRED portaudio-2.0)
pkg_check_modules(FFTW3F    REQUIRED fftw3f)
pkg_check_modules(SDL2      REQUIRED sdl2)

find_package(OpenGL REQUIRED)

include(FetchContent)

FetchContent_Declare(imgui
    GIT_REPOSITORY https://github.com/ocornut/imgui.git
    GIT_TAG        v1.91.5
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(imgui)

FetchContent_Declare(implot
    GIT_REPOSITORY https://github.com/epezent/implot.git
    GIT_TAG        v0.17
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(implot)

# ── ImGui static library (SDL2 + OpenGL3 backend) ────────────────────────────

add_library(imgui_lib STATIC
    ${imgui_SOURCE_DIR}/imgui.cpp
    ${imgui_SOURCE_DIR}/imgui_draw.cpp
    ${imgui_SOURCE_DIR}/imgui_tables.cpp
    ${imgui_SOURCE_DIR}/imgui_widgets.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_sdl2.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_opengl3.cpp
)
target_include_directories(imgui_lib PUBLIC
    ${imgui_SOURCE_DIR}
    ${imgui_SOURCE_DIR}/backends
    ${SDL2_INCLUDE_DIRS}
)
target_link_libraries(imgui_lib PUBLIC ${SDL2_LIBRARIES} OpenGL::GL)
target_compile_options(imgui_lib PRIVATE ${SDL2_CFLAGS_OTHER})

# ── ImPlot static library ─────────────────────────────────────────────────────

add_library(implot_lib STATIC
    ${implot_SOURCE_DIR}/implot.cpp
    ${implot_SOURCE_DIR}/implot_items.cpp
)
target_include_directories(implot_lib PUBLIC ${implot_SOURCE_DIR})
target_link_libraries(implot_lib PUBLIC imgui_lib)

# ── Application sources ───────────────────────────────────────────────────────

set(CORE_SOURCES
    src/AudioCapture.cpp
    src/MelFilterbank.cpp
    src/DSP.cpp
    src/Visualizer.cpp
    src/LEDOutput.cpp
    src/GUI.cpp
)

add_executable(led_visualizer src/main.cpp ${CORE_SOURCES})

target_include_directories(led_visualizer PRIVATE
    src/
    ${PORTAUDIO_INCLUDE_DIRS}
    ${FFTW3F_INCLUDE_DIRS}
    ${SDL2_INCLUDE_DIRS}
)

target_link_directories(led_visualizer PRIVATE
    ${PORTAUDIO_LIBRARY_DIRS}
    ${FFTW3F_LIBRARY_DIRS}
    ${SDL2_LIBRARY_DIRS}
)

target_link_libraries(led_visualizer PRIVATE
    ${PORTAUDIO_LIBRARIES}
    ${FFTW3F_LIBRARIES}
    imgui_lib
    implot_lib
    OpenGL::GL
    "-framework CoreAudio"
    "-framework AudioToolbox"
    "-framework AudioUnit"
    "-framework CoreFoundation"
    "-framework CoreServices"
)

target_compile_options(led_visualizer PRIVATE
    ${PORTAUDIO_CFLAGS_OTHER}
    -Wall -Wextra -O2
)

if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    target_link_options(led_visualizer PRIVATE -Wl,-no_warn_duplicate_libraries)
endif()
