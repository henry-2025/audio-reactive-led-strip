/* OpenGL Area
 *
 * GtkGLArea is a widget that allows custom drawing using OpenGL calls.
 */

// compiling with: g++ gl_draw_area.cpp `pkg-config --cflags gtk+-3.0`
// \ `pkg-config --libs gtk+-3.0` -lepoxy

#include <gl/glew.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>
#include <gtk/gtk.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#include <iostream>

unsigned int WIDTH = 800;
unsigned int HEIGHT = 600;

using glm::lookAt;
using glm::mat4;
using glm::perspective;
using glm::rotate;
using glm::vec3;

const GLchar *VERTEX_SOURCE =
    "#version 330\n"
    "in vec3 position;\n"
    "in vec3 normal;\n"
    "out vec3 transformedNormal;\n"
    "out vec3 originalNormal;\n"
    "uniform mat4 projection;\n"
    "uniform mat4 view;\n"
    "uniform mat4 model;\n"
    "void main(){\n"
    "    gl_Position =  projection * view * model * vec4(position, 1.0);\n"
    "    mat3 normalMatrix = transpose(inverse(mat3(view * model)));\n"
    "    transformedNormal = normalMatrix * normal;\n"
    "    originalNormal = abs(normal);\n"
    "}\n";

const GLchar *FRAGMENT_SOURCE =
    "#version 330\n"
    "in vec3 transformedNormal;\n"
    "in vec3 originalNormal;\n"
    "out vec4 outputColor;\n"
    "void main() {\n"
    "vec3 color = originalNormal;\n"
    "float lighting = abs(dot(transformedNormal, vec3(0,0,-1)));\n"
    "outputColor = vec4(color * lighting, 1.0f);\n" // constant white
    "}";

/* the GtkGLArea widget */
static GtkWidget *gl_area = NULL;

/* The object we are drawing */
static const GLfloat vertex_data[] = {
    1.0,  -1.0, -1.0, 0.0,  -1.0, 0.0,  1.0,  -1.0, 1.0,  0.0,
    -1.0, 0.0,  -1.0, -1.0, 1.0,  0.0,  -1.0, 0.0,  1.0,  -1.0,
    -1.0, 0.0,  -1.0, 0.0,  -1.0, -1.0, 1.0,  0.0,  -1.0, 0.0,
    -1.0, -1.0, -1.0, 0.0,  -1.0, 0.0,

    -1.0, 1.0,  1.0,  0.0,  1.0,  0.0,  1.0,  1.0,  1.0,  0.0,
    1.0,  0.0,  1.0,  1.0,  -1.0, 0.0,  1.0,  0.0,  -1.0, 1.0,
    1.0,  0.0,  1.0,  0.0,  1.0,  1.0,  -1.0, 0.0,  1.0,  0.0,
    -1.0, 1.0,  -1.0, 0.0,  1.0,  0.0,

    -1.0, -1.0, -1.0, -1.0, 0.0,  0.0,  -1.0, -1.0, 1.0,  -1.0,
    0.0,  0.0,  -1.0, 1.0,  -1.0, -1.0, 0.0,  0.0,  -1.0, -1.0,
    1.0,  -1.0, 0.0,  0.0,  -1.0, 1.0,  1.0,  -1.0, 0.0,  0.0,
    -1.0, 1.0,  -1.0, -1.0, 0.0,  0.0,

    -1.0, -1.0, 1.0,  0.0,  0.0,  1.0,  1.0,  -1.0, 1.0,  0.0,
    0.0,  1.0,  -1.0, 1.0,  1.0,  0.0,  0.0,  1.0,  1.0,  -1.0,
    1.0,  0.0,  0.0,  1.0,  1.0,  1.0,  1.0,  0.0,  0.0,  1.0,
    -1.0, 1.0,  1.0,  0.0,  0.0,  1.0,

    1.0,  1.0,  -1.0, 0.0,  0.0,  -1.0, 1.0,  -1.0, -1.0, 0.0,
    0.0,  -1.0, -1.0, -1.0, -1.0, 0.0,  0.0,  -1.0, 1.0,  1.0,
    -1.0, 0.0,  0.0,  -1.0, -1.0, -1.0, -1.0, 0.0,  0.0,  -1.0,
    -1.0, 1.0,  -1.0, 0.0,  0.0,  -1.0,

    1.0,  1.0,  1.0,  1.0,  0.0,  0.0,  1.0,  -1.0, 1.0,  1.0,
    0.0,  0.0,  1.0,  -1.0, -1.0, 1.0,  0.0,  0.0,  1.0,  1.0,
    1.0,  1.0,  0.0,  0.0,  1.0,  -1.0, -1.0, 1.0,  0.0,  0.0,
    1.0,  1.0,  -1.0, 1.0,  0.0,  0.0

};

long current_frame = 0.0;
long delta_time = 0.0;
GDateTime *last_frame;
int dt = 0;

static GLuint position_buffer;
static GLuint program;
static GLuint vao;

mat4 model = mat4(1.0);

/* Create and compile a shader */
static GLuint create_shader(int type) {
  GLuint shader;
  int status;
  shader = glCreateShader(type);
  if (type == GL_FRAGMENT_SHADER) {
    glShaderSource(shader, 1, &FRAGMENT_SOURCE, NULL);
  }
  if (type == GL_VERTEX_SHADER) {
    glShaderSource(shader, 1, &VERTEX_SOURCE, NULL);
  }
  glCompileShader(shader);

  glGetShaderiv(shader, GL_COMPILE_STATUS, &status);
  if (status == GL_FALSE) {
    int log_len;
    char *buffer;
    glGetShaderiv(shader, GL_INFO_LOG_LENGTH, &log_len);
    buffer = (char *)g_malloc(log_len + 1);
    glGetShaderInfoLog(shader, log_len, NULL, buffer);
    g_warning("Compile failure in %s shader:\n%s",
              type == GL_VERTEX_SHADER ? "vertex" : "fragment", buffer);
    g_free(buffer);
    glDeleteShader(shader);
    return 0;
  }

  return shader;
}

/* We need to set up our state when we realize the GtkGLArea widget */
static void realize(GtkWidget *widget) {

  GdkGLContext *context;
  gtk_gl_area_set_required_version(GTK_GL_AREA(widget), 3, 3);
  gtk_gl_area_make_current(GTK_GL_AREA(widget));
  if (gtk_gl_area_get_error(GTK_GL_AREA(widget)) != NULL)
    return;
  context = gtk_gl_area_get_context(GTK_GL_AREA(widget));

  GLenum glew_error = glewInit();
  if (glew_error != GLEW_OK) {
    g_print("Unable to initialize glew %s\n", glewGetErrorString(glew_error));
    exit(1);
  }

  /* We only use one VAO, so we always keep it bound */
  glGenVertexArrays(1, &vao);
  glBindVertexArray(vao);

  /* This is the buffer that holds the vertices */
  glGenBuffers(1, &position_buffer);
  glBindBuffer(GL_ARRAY_BUFFER, position_buffer);
  glBufferData(GL_ARRAY_BUFFER, sizeof(vertex_data), vertex_data,
               GL_STATIC_DRAW);
  glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, 6 * sizeof(float), (void *)0);
  glEnableVertexAttribArray(0);
  glVertexAttribPointer(1, 3, GL_FLOAT, GL_FALSE, 6 * sizeof(float),
                        (void *)(3 * sizeof(float)));
  glEnableVertexAttribArray(1);
  glBindBuffer(GL_ARRAY_BUFFER, 0);

  GLuint vertex, fragment;
  int status;
  vertex = create_shader(GL_VERTEX_SHADER);

  if (vertex == 0) {
    return;
  }

  fragment = create_shader(GL_FRAGMENT_SHADER);

  if (fragment == 0) {
    glDeleteShader(vertex);
    return;
  }

  program = glCreateProgram();
  glAttachShader(program, vertex);
  glAttachShader(program, fragment);

  glLinkProgram(program);

  glGetProgramiv(program, GL_LINK_STATUS, &status);
  if (status == GL_FALSE) {
    int log_len;
    char *buffer;

    glGetProgramiv(program, GL_INFO_LOG_LENGTH, &log_len);

    buffer = (char *)g_malloc(log_len + 1);
    glGetProgramInfoLog(program, log_len, NULL, buffer);

    g_warning("Linking failure:\n%s", buffer);

    g_free(buffer);

    glDeleteProgram(program);
    program = 0;

    glDeleteShader(vertex);
    glDeleteShader(fragment);

    return;
  }

  glDetachShader(program, vertex);
  glDetachShader(program, fragment);

  glEnable(GL_CULL_FACE);
  glFrontFace(GL_CCW);
  glCullFace(GL_BACK);
  glEnable(GL_DEPTH_TEST);
}

/* We should tear down the state when unrealizing */
static void unrealize(GtkWidget *widget) {
  gtk_gl_area_make_current(GTK_GL_AREA(widget));

  if (gtk_gl_area_get_error(GTK_GL_AREA(widget)) != NULL)
    return;

  glDeleteBuffers(1, &position_buffer);
  glDeleteProgram(program);
}

static void draw_box(long delta_time) {
  /* Use our shaders */
  glUseProgram(program);

  model = rotate(model, (float)delta_time / 1000, vec3(1, 1, 0));
  glUniformMatrix4fv(glGetUniformLocation(program, "model"), 1, GL_FALSE,
                     &model[0][0]);
  vec3 position = vec3(0, 0, 5);
  vec3 front = vec3(0, 0, -1);
  vec3 up = vec3(0, 1, 0);
  mat4 view = lookAt(position, position + front, up);
  glUniformMatrix4fv(glGetUniformLocation(program, "view"), 1, GL_FALSE,
                     &view[0][0]);
  mat4 projection =
      perspective(45.0, double(WIDTH) / double(HEIGHT), 0.1, 100.0);
  glUniformMatrix4fv(glGetUniformLocation(program, "projection"), 1, GL_FALSE,
                     &projection[0][0]);

  glBindVertexArray(vao);
  /* Use the vertices in our buffer */

  /* Draw the three vertices as a triangle */
  glDrawArrays(GL_TRIANGLES, 0, 36);

  /* We finished using the buffers and program */
  glBindVertexArray(0);
  glDisableVertexAttribArray(0);
  glBindBuffer(GL_ARRAY_BUFFER, 0);
  glUseProgram(0);
}

static gboolean render(GtkGLArea *area, GdkGLContext *context) {
  // GDateTime *date_time;

  // date_time = g_date_time_new_now_local();
  // current_frame = g_date_time_get_microsecond(date_time);
  // delta_time = g_date_time_difference(date_time, last_frame) / 1000;
  // last_frame = date_time;

  // if (gtk_gl_area_get_error(area) != NULL)
  //   return FALSE;

  /* Clear the viewport */
  glClearColor(0.0, 0.0, 0.0, 0.0);
  glClear(GL_COLOR_BUFFER_BIT);
  ///* Draw our object */
  // draw_box(delta_time);
  ///* Flush the contents of the pipeline */
  // glFlush();
  // gtk_gl_area_queue_render(area);
  return TRUE;
}

static void on_axis_value_change(void) { gtk_widget_queue_draw(gl_area); }

static void activate(GtkApplication *app, gpointer user_data) {
  /* initialize gtk */
  /* Create new top level window. */
  GtkWidget *window, *box, *slider;

  window = gtk_application_window_new(app);
  gtk_window_set_title(GTK_WINDOW(window), "Reactive Desktop");
  gtk_window_set_default_size(GTK_WINDOW(window), WIDTH, HEIGHT);

  box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
  // TODO: create a dual slider widget when you have the time
  slider = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 0, 10, 0.1);
  gtk_window_set_child(GTK_WINDOW(window), box);
  gl_area = gtk_gl_area_new();
  /* We need to initialize and free GL resources, so we use
   * the realize and unrealize signals on the widget
   */
   g_signal_connect(gl_area, "realize", G_CALLBACK(realize), NULL);
   g_signal_connect(gl_area, "unrealize", G_CALLBACK(unrealize), NULL);

  /* The main "draw" call for GtkGLArea */
  g_signal_connect(gl_area, "render", G_CALLBACK(render), NULL);

  gtk_box_append(GTK_BOX(box), slider);
  gtk_box_append(GTK_BOX(box), gl_area);
  gtk_window_present(GTK_WINDOW(window));
}

int main(int argc, char **argv) {
  GtkApplication *app;
  int status;

  app = gtk_application_new("org.henry-2025.reactive",
                            G_APPLICATION_DEFAULT_FLAGS);
  g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
  status = g_application_run(G_APPLICATION(app), argc, argv);
  g_object_unref(app);

  return status;
}
