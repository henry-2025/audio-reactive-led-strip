#include "GUI.h"
#include "Config.h"

#include <imgui.h>
#include <imgui_impl_sdl2.h>
#include <imgui_impl_opengl3.h>
#include <implot.h>

#include <SDL2/SDL.h>
#include <SDL2/SDL_opengl.h>

#include <algorithm>
#include <cassert>
#include <cmath>
#include <stdexcept>

// ── Helpers ───────────────────────────────────────────────────────────────────

static constexpr ImVec4 kCyan   = {0.086f, 0.859f, 0.922f, 1.0f}; // #16dbeb
static constexpr ImVec4 kRed    = {1.000f, 0.118f, 0.118f, 0.784f};
static constexpr ImVec4 kGreen  = {0.118f, 1.000f, 0.118f, 0.784f};
static constexpr ImVec4 kBlue   = {0.118f, 0.118f, 1.000f, 0.784f};

static inline SDL_Window*   toSDLWindow(void* p) { return static_cast<SDL_Window*>(p); }
static inline SDL_GLContext toGLContext(void* p)  { return static_cast<SDL_GLContext>(p); }

// ── Construction / destruction ────────────────────────────────────────────────

GUI::GUI() {
    if (SDL_Init(SDL_INIT_VIDEO) != 0)
        throw std::runtime_error(SDL_GetError());

    SDL_GL_SetAttribute(SDL_GL_CONTEXT_FLAGS,        SDL_GL_CONTEXT_FORWARD_COMPATIBLE_FLAG);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 2);
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    SDL_GL_SetAttribute(SDL_GL_DEPTH_SIZE, 24);
    SDL_GL_SetAttribute(SDL_GL_STENCIL_SIZE, 8);

    SDL_Window* win = SDL_CreateWindow(
        "Audio Reactive LED Strip",
        SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
        1000, 700,
        SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_ALLOW_HIGHDPI
    );
    if (!win) throw std::runtime_error(SDL_GetError());
    window_ = win;

    SDL_GLContext ctx = SDL_GL_CreateContext(win);
    if (!ctx) throw std::runtime_error(SDL_GetError());
    SDL_GL_MakeCurrent(win, ctx);
    SDL_GL_SetSwapInterval(1); // VSync — naturally paces to ~60 FPS
    gl_ctx_ = ctx;

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImPlot::CreateContext();

    ImGui::StyleColorsDark();
    // Tint active/hovered elements with the project's cyan accent.
    ImGuiStyle& s = ImGui::GetStyle();
    s.WindowRounding = 4.0f;
    s.FrameRounding  = 3.0f;
    s.Colors[ImGuiCol_SliderGrab]        = kCyan;
    s.Colors[ImGuiCol_SliderGrabActive]  = kCyan;
    s.Colors[ImGuiCol_CheckMark]         = kCyan;

    ImGui_ImplSDL2_InitForOpenGL(win, ctx);
    ImGui_ImplOpenGL3_Init("#version 150");
}

GUI::~GUI() {
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplSDL2_Shutdown();
    ImPlot::DestroyContext();
    ImGui::DestroyContext();
    if (gl_ctx_) SDL_GL_DeleteContext(toGLContext(gl_ctx_));
    if (window_) SDL_DestroyWindow(toSDLWindow(window_));
    SDL_Quit();
}

// ── Frame lifecycle ───────────────────────────────────────────────────────────

bool GUI::pollEvents() {
    SDL_Event e;
    while (SDL_PollEvent(&e)) {
        ImGui_ImplSDL2_ProcessEvent(&e);
        if (e.type == SDL_QUIT) return false;
        if (e.type == SDL_WINDOWEVENT &&
            e.window.event == SDL_WINDOWEVENT_CLOSE &&
            e.window.windowID == SDL_GetWindowID(toSDLWindow(window_)))
            return false;
    }
    return true;
}

void GUI::beginFrame() {
    ImGui_ImplOpenGL3_NewFrame();
    ImGui_ImplSDL2_NewFrame();
    ImGui::NewFrame();
}

void GUI::endFrame() {
    ImGui::Render();
    SDL_Window* win = toSDLWindow(window_);
    int w, h;
    SDL_GL_GetDrawableSize(win, &w, &h);
    glViewport(0, 0, w, h);
    glClearColor(0.10f, 0.10f, 0.10f, 1.0f);
    glClear(GL_COLOR_BUFFER_BIT);
    ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
    SDL_GL_SwapWindow(win);
}

// ── Frequency accessors ───────────────────────────────────────────────────────

float GUI::minHz() const {
    return freq_lo_ * freq_lo_ * (Config::MIC_RATE / 2.0f);
}

float GUI::maxHz() const {
    return freq_hi_ * freq_hi_ * (Config::MIC_RATE / 2.0f);
}

// ── Custom double-handle frequency slider ─────────────────────────────────────
//
// Handle position t maps to freq = t^2 * (MIC_RATE/2), matching the Python
// TickSliderItem quadratic scale. Both handles share one InvisibleButton hit
// area; drag_lo_ records which handle was nearest at the moment of click.

bool GUI::freqRangeSlider() {
    const float  w  = ImGui::GetContentRegionAvail().x;
    const float  h  = 28.0f;
    const float  R  = 8.0f;   // handle radius
    ImVec2       p  = ImGui::GetCursorScreenPos();
    ImDrawList*  dl = ImGui::GetWindowDrawList();

    float track_y = p.y + h * 0.5f;
    float lo_x    = p.x + freq_lo_ * w;
    float hi_x    = p.x + freq_hi_ * w;

    // Track background
    dl->AddRectFilled({p.x,  track_y - 2.0f}, {p.x + w, track_y + 2.0f},
                      IM_COL32(80, 80, 80, 255), 2.0f);
    // Highlighted range between handles
    dl->AddRectFilled({lo_x, track_y - 2.0f}, {hi_x,    track_y + 2.0f},
                      IM_COL32(22, 219, 235, 180), 2.0f);
    // Handles (drawn after range so they appear on top)
    dl->AddCircleFilled({lo_x, track_y}, R, IM_COL32(22, 219, 235, 255));
    dl->AddCircleFilled({hi_x, track_y}, R, IM_COL32(22, 219, 235, 255));
    // Handle outlines for contrast
    dl->AddCircle({lo_x, track_y}, R, IM_COL32(255, 255, 255, 120));
    dl->AddCircle({hi_x, track_y}, R, IM_COL32(255, 255, 255, 120));

    // Invisible hit area spanning the full track
    ImGui::SetCursorScreenPos(p);
    ImGui::InvisibleButton("##freqslider", {w, h});

    bool changed = false;

    if (ImGui::IsItemActivated()) {
        // Decide at click-start which handle to drag based on proximity
        float mx = (ImGui::GetIO().MousePos.x - p.x) / w;
        drag_lo_ = std::abs(mx - freq_lo_) < std::abs(mx - freq_hi_);
    }
    if (ImGui::IsItemActive()) {
        float mx = std::clamp((ImGui::GetIO().MousePos.x - p.x) / w, 0.0f, 1.0f);
        constexpr float MIN_GAP = 0.02f;
        if (drag_lo_) freq_lo_ = std::min(mx, freq_hi_ - MIN_GAP);
        else          freq_hi_ = std::max(mx, freq_lo_ + MIN_GAP);
        changed = true;
    }

    // Advance cursor below the widget
    ImGui::SetCursorScreenPos({p.x, p.y + h + ImGui::GetStyle().ItemSpacing.y});
    return changed;
}

// ── Main draw call ────────────────────────────────────────────────────────────

bool GUI::draw(const float* mel, Visualizer& viz, const PixelFrame& pixels, float fps) {
    constexpr int N  = Config::N_FFT_BINS;
    constexpr int NP = Config::N_PIXELS;

    // Full-screen, borderless ImGui window
    ImGui::SetNextWindowPos({0, 0});
    ImGui::SetNextWindowSize(ImGui::GetIO().DisplaySize);
    ImGui::Begin("##root", nullptr,
        ImGuiWindowFlags_NoTitleBar    | ImGuiWindowFlags_NoResize  |
        ImGuiWindowFlags_NoMove        | ImGuiWindowFlags_NoScrollbar |
        ImGuiWindowFlags_NoBringToFrontOnFocus);

    // Reserve ~120 px for the controls panel at the bottom
    float avail_h    = ImGui::GetContentRegionAvail().y;
    float plot_h     = std::max(60.0f, (avail_h - 140.0f) * 0.5f);

    // ── Mel spectrum plot ─────────────────────────────────────────────────────

    // Smooth mel values for display (matches Python fft_plot_filter)
    fft_plot_filter_.update(mel, N);

    // X-axis in Hz, evenly spaced between the current frequency limits
    float mel_x[N], mel_y[N];
    float hz_lo = minHz(), hz_hi = maxHz();
    float bar_w = (hz_hi - hz_lo) / N * 0.85f;
    for (int i = 0; i < N; ++i) {
        mel_x[i] = hz_lo + (hz_hi - hz_lo) * i / (N - 1);
        mel_y[i] = fft_plot_filter_.value[i];
    }

    ImPlot::PushStyleColor(ImPlotCol_Fill, kCyan);
    if (ImPlot::BeginPlot("Filterbank Output", {-1, plot_h})) {
        ImPlot::SetupAxes("Frequency (Hz)", nullptr);
        ImPlot::SetupAxisLimits(ImAxis_Y1, -0.05, 1.25, ImGuiCond_Always);
        ImPlot::PlotBars("##mel", mel_x, mel_y, N, bar_w);
        ImPlot::EndPlot();
    }
    ImPlot::PopStyleColor();

    // ── LED pixel plot ────────────────────────────────────────────────────────

    float px_x[NP], px_r[NP], px_g[NP], px_b[NP];
    for (int i = 0; i < NP; ++i) {
        px_x[i] = static_cast<float>(i);
        px_r[i] = static_cast<float>(pixels[0][i]);
        px_g[i] = static_cast<float>(pixels[1][i]);
        px_b[i] = static_cast<float>(pixels[2][i]);
    }

    if (ImPlot::BeginPlot("LED Output", {-1, plot_h})) {
        ImPlot::SetupAxes("Pixel", nullptr);
        ImPlot::SetupAxisLimits(ImAxis_Y1, -5, 265, ImGuiCond_Always);
        ImPlot::SetNextLineStyle(kRed,   2.0f); ImPlot::PlotLine("R", px_x, px_r, NP);
        ImPlot::SetNextLineStyle(kGreen, 2.0f); ImPlot::PlotLine("G", px_x, px_g, NP);
        ImPlot::SetNextLineStyle(kBlue,  2.0f); ImPlot::PlotLine("B", px_x, px_b, NP);
        ImPlot::EndPlot();
    }

    // ── Controls ──────────────────────────────────────────────────────────────

    ImGui::Separator();
    ImGui::Text("FPS: %.1f", fps);
    ImGui::Text("Frequency range: %.0f – %.0f Hz", hz_lo, hz_hi);

    bool freq_changed = freqRangeSlider();

    ImGui::Spacing();

    // Effect selector — active button highlighted with cyan
    auto effectBtn = [&](const char* label, Visualizer::Effect e) {
        bool active = viz.effect() == e;
        if (active) ImGui::PushStyleColor(ImGuiCol_Button, kCyan);
        if (ImGui::Button(label, {90, 0})) viz.setEffect(e);
        if (active) ImGui::PopStyleColor();
        ImGui::SameLine();
    };
    effectBtn("Energy",   Visualizer::Effect::Energy);
    effectBtn("Scroll",   Visualizer::Effect::Scroll);
    effectBtn("Spectrum", Visualizer::Effect::Spectrum);

    ImGui::End();
    return freq_changed;
}
