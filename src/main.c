#include <gtk/gtk.h>

#include "reactive_app.h"

int
main (int argc, char *argv[])
{
  return g_application_run (G_APPLICATION (reactive_app_new ()), argc, argv);
}
