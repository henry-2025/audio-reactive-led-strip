#ifndef REACTIVE_APP_H
#define REACTIVE_APP_H

#include <gtk/gtk.h>

G_BEGIN_DECLS

#define REACTIVE_TYPE_APP (reactive_app_get_type ())

G_DECLARE_FINAL_TYPE (ReactiveApp, reactive_app, REACTIVE, APP, GtkApplication)

GtkApplication *reactive_app_new (void);

G_END_DECLS

#endif
