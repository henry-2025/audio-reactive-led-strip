#include "gui.h"
#include "config.h"
#include <GL/glew.h>
#include <gtk/gtk.h>
#include <stdio.h>

static void realize(GtkWidget *widget) {
  GdkGLContext *context;
  gtk_gl_area_set_required_version(GTK_GL_AREA(widget), 3, 3);
  gtk_gl_area_make_current(GTK_GL_AREA(widget));
  if (gtk_gl_area_get_error(GTK_GL_AREA(widget)) != NULL)
    return;
  context = gtk_gl_area_get_context(GTK_GL_AREA(widget));

  GLenum err = glewInit();

  if (err != GLEW_OK) {
    printf("Error initializing glew: %s", glewGetString(err));
    return;
  }
}

static void unrealize(GtkWidget *widget) {
  gtk_gl_area_get_context(GTK_GL_AREA(widget));

  if (gtk_gl_area_get_error(GTK_GL_AREA(widget)) != NULL)
    return;
}

static gboolean render(GtkGLArea *area, GdkGLContext *context) {
  glClearColor(0.0, 0.0, 0.0, 0.0);
  glClear(GL_COLOR_BUFFER_BIT);

  gtk_gl_area_queue_render(area);
  return TRUE;
}

static void activate(GtkApplication *app, gpointer user_data) {
  /* initialize gtk */
  /* Create new top level window. */
  GtkWidget *window, *box, *slider, *gl_area;

  window = gtk_application_window_new(app);
  gtk_window_set_title(GTK_WINDOW(window), "Reactive Desktop");
  gtk_window_set_default_size(GTK_WINDOW(window), WINDOW_WIDTH, WINDOW_HEIGHT);

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

int run_app(int argc, char **argv) {
  GtkApplication *app;
  int status;

  app = gtk_application_new("org.henry-2025.reactive",
                            G_APPLICATION_DEFAULT_FLAGS);
  g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
  status = g_application_run(G_APPLICATION(app), argc, argv);
  g_object_unref(app);

  return status;
}
