// SPDX-License-Identifier: GPL-3.0-or-later
use gettextrs::gettext;
use log::{debug, info};

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::window::Window;

mod imp {
    use super::*;

    use adw::subclass::prelude::*;

    #[derive(Debug, Default)]
    pub struct Application;

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "Application";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {
        fn constructed(&self) {
            log::debug!("Application::constructed");
            self.parent_constructed();

            self.obj().add_main_option(
                "debug",
                glib::Char::from(b'd'),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                &gettext("Enable debug messages"),
                None,
            );
        }
    }

    impl ApplicationImpl for Application {
        fn handle_local_options(&self, options: &glib::VariantDict) -> glib::ExitCode {
            let is_debug = options.lookup::<bool>("debug").unwrap().unwrap_or_default()
                || !glib::log_writer_default_would_drop(glib::LogLevel::Debug, Some("citations"));
            if is_debug {
                tracing_subscriber::fmt()
                    .with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG)
                    .init();
            } else {
                tracing_subscriber::fmt::init();
            }

            log::debug!("Application::handle_local_options");

            self.parent_handle_local_options(options)
        }

        fn activate(&self) {
            debug!("Application::activate");
            self.parent_activate();

            let window = Window::new(&self.obj());
            let group = gtk::WindowGroup::new();
            group.add_window(&window);

            window.present();
        }

        fn startup(&self) {
            debug!("Application::startup");
            self.parent_startup();
            let app = self.obj();

            gsv::init();

            // Set icons for shell
            gtk::Window::set_default_icon_name(APP_ID);

            app.setup_gactions();
            app.setup_accels();
        }

        fn shutdown(&self) {
            self.parent_shutdown();

            gsv::finalize();
        }

        fn open(&self, files: &[gio::File], _hint: &str) {
            debug!("Application::open");
            for file in files {
                let window = Window::new(&self.obj());
                let group = gtk::WindowGroup::new();
                group.add_window(&window);
                window.present();

                glib::spawn_future_local(glib::clone!(
                    #[strong]
                    window,
                    #[strong]
                    file,
                    async move {
                        if let Err(err) = window.open(&file).await {
                            log::error!("Could not open {}: {err}", file.uri());
                        }
                    }
                ));
            }
        }
    }

    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Default for Application {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", APP_ID)
            .property("resource-base-path", "/org/gnome/World/Citations/")
            .property("flags", gio::ApplicationFlags::HANDLES_OPEN)
            .build()
    }
}

impl Application {
    fn setup_gactions(&self) {
        let actions = [
            gio::ActionEntryBuilder::new("quit")
                .activate(|app: &Self, _, _| {
                    // This is needed to trigger the delete event and saving the window state
                    if let Some(window) = app.active_window() {
                        window.close();
                    }
                    app.quit();
                })
                .build(),
            gio::ActionEntryBuilder::new("new-window")
                .activate(|app, _, _| {
                    let window = Window::new(app);
                    let group = gtk::WindowGroup::new();
                    group.add_window(&window);
                    window.present();
                })
                .build(),
        ];

        self.add_action_entries(actions);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.new-window", &["<Control>n"]);
        self.set_accels_for_action("win.open('')", &["<Control>o"]);
        self.set_accels_for_action("win.save", &["<Control>s"]);
        self.set_accels_for_action("win.search", &["<Control>f"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
    }

    pub fn run(&self) -> glib::ExitCode {
        info!("Citations ({})", APP_ID);
        info!("Version: {} ({})", VERSION, PROFILE);
        info!("Datadir: {}", PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}
