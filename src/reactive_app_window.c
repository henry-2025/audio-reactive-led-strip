#include "reactive_app_window.h"
#include "config.h"
#include <epoxy/gl.h>
#include <gtk/gtk.h>
#include <stdio.h>

struct _ReactiveAppWindow {
  GtkApplicationWindow parent_instance;

  /* the adjustments we use to control the rotation angles */
  GtkAdjustment *x_adjustment;
  // GtkAdjustment *y_adjustment;
  // GtkAdjustment *z_adjustment;

  /* our GL rendering widget */
  GtkWidget *gl_drawing_area;

  /* GL objects */
  // guint vao;
  // guint program;
  // guint mvp_location;
  // guint position_index;
  // guint color_index;
};

struct _ReactiveAppWindowClass {
  GtkApplicationWindowClass parent_class;
};

G_DEFINE_TYPE(ReactiveAppWindow, reactive_app_window,
              GTK_TYPE_APPLICATION_WINDOW)

static void gl_init(ReactiveAppWindow *self) {
  char *title;
  const char *renderer;

  /* we need to ensure that the GdkGLContext is set before calling GL API */
  gtk_gl_area_make_current(GTK_GL_AREA(self->gl_drawing_area));

  /* if the GtkGLArea is in an error state we don't do anything */
  if (gtk_gl_area_get_error(GTK_GL_AREA(self->gl_drawing_area)) != NULL)
    return;

  renderer = (char *)glGetString(GL_RENDERER);
  title = g_strdup_printf("glarea on %s", renderer ? renderer : "Unknown");
  gtk_window_set_title(GTK_WINDOW(self), title);
  g_free(title);
}

static void gl_cleanup(ReactiveAppWindow *self) {
  /* we need to ensure that the GdkGLContext is set before calling GL API */
  gtk_gl_area_make_current(GTK_GL_AREA(self->gl_drawing_area));

  /* skip everything if we're in error state */
  if (gtk_gl_area_get_error(GTK_GL_AREA(self->gl_drawing_area)) != NULL)
    return;

  /* destroy all the resources we created */
  // none right now, since we are just doing blank render
}

static void draw_graph(ReactiveAppWindow *self) {
  // nothing to do right now
}

static gboolean gl_draw(ReactiveAppWindow *self) {

  /* clear the viewport; the viewport is automatically resized when
   * the GtkGLArea gets a new size allocation
   */
  glClearColor(0.5, 0.5, 0.5, 1.0);
  glClear(GL_COLOR_BUFFER_BIT);

  /* draw our object */
  draw_graph(self);

  /* flush the contents of the pipeline */
  glFlush();

  return FALSE;
}

static void
reactive_app_window_class_init (ReactiveAppWindowClass *class)
{
  GtkWidgetClass *widget_class = GTK_WIDGET_CLASS (class);

  gtk_widget_class_set_template_from_resource (widget_class, "/io/bassi/glarea/glarea-app-window.ui");

  gtk_widget_class_bind_template_child (widget_class, ReactiveAppWindow, gl_drawing_area);
  gtk_widget_class_bind_template_child (widget_class, ReactiveAppWindow, x_adjustment);
  gtk_widget_class_bind_template_callback (widget_class, gl_init);
  gtk_widget_class_bind_template_callback (widget_class, gl_draw);
  gtk_widget_class_bind_template_callback (widget_class, gl_cleanup);
}

static void
reactive_app_window_init (ReactiveAppWindow *self)
{
  gtk_widget_init_template (GTK_WIDGET (self));

  /* reset the mvp matrix */
  //init_mvp (self->mvp);

  gtk_window_set_icon_name (GTK_WINDOW (self), "glarea");
}

GtkWidget *
reactive_app_window_new (ReactiveApp *app)
{
  return g_object_new (reactive_app_window_get_type (), "application", app, NULL);
}
