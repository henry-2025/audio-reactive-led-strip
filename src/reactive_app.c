#include "reactive_app.h"
#include "reactive_app_window.h"

struct _ReactiveApp
{
  GtkApplication parent_instance;

  GtkWidget *window;
};

struct _ReactiveAppClass
{
  GtkApplicationClass parent_class;
};

G_DEFINE_TYPE (ReactiveApp, reactive_app, GTK_TYPE_APPLICATION)

static void
quit_activated (GSimpleAction *action,
                GVariant      *parameter,
                gpointer       app)
{
  g_application_quit (G_APPLICATION (app));
}

static GActionEntry app_entries[] =
{
  { "quit", quit_activated, NULL, NULL, NULL }
};

static void
reactive_app_startup (GApplication *app)
{
  GtkBuilder *builder;
  GMenuModel *app_menu;

  G_APPLICATION_CLASS (reactive_app_parent_class)->startup (app);

  g_action_map_add_action_entries (G_ACTION_MAP (app),
                                   app_entries, G_N_ELEMENTS (app_entries),
                                   app);

  builder = gtk_builder_new_from_resource ("/io/bassi/reactive/reactive_app_menu.ui");
  app_menu = G_MENU_MODEL (gtk_builder_get_object (builder, "appmenu"));
  //gtk_application_set_app_menu (GTK_APPLICATION (app), app_menu);
  g_object_unref (builder);
}

static void
reactive_app_activate (GApplication *app)
{
  ReactiveApp *self = REACTIVE_APP (app);

  if (self->window == NULL)
    self->window = reactive_app_window_new (REACTIVE_APP (app));

  gtk_window_present (GTK_WINDOW (self->window));
}


static void
reactive_app_class_init (ReactiveAppClass *class)
{
  GApplicationClass *app_class = G_APPLICATION_CLASS (class);

  app_class->startup = reactive_app_startup;
  app_class->activate = reactive_app_activate;
}

static void
reactive_app_init (ReactiveApp *self)
{
}

GtkApplication *
reactive_app_new (void)
{
  return g_object_new (reactive_app_get_type (), "application-id", "io.henry-2025.Reactive", NULL);
}
