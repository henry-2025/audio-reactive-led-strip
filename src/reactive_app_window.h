#ifndef GUI_H
#define GUI_H

#include <gtk/gtk.h>
#include "reactive_app.h"

G_BEGIN_DECLS

#define REACTIVE_TYPE_APP_WINDOW (reactive_app_window_get_type ())

G_DECLARE_FINAL_TYPE (ReactiveAppWindow, reactive_app_window, REACTIVE, APP_WINDOW, GtkApplicationWindow)

GtkWidget *reactive_app_window_new (ReactiveApp *app);

G_END_DECLS

#endif
