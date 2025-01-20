// SPDX-License-Identifier: GPL-3.0-or-later
use adw::prelude::*;
use gettextrs::gettext;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use crate::application::Application;
use crate::config::{APP_ID, PROFILE, VERSION};
use crate::utils;
use cratebibtex::format;

mod imp {
    use super::*;

    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use std::cell::{Cell, OnceCell, RefCell};

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/org/gnome/World/Citations/ui/window.ui")]
    pub struct Window {
        pub bib: OnceCell<cratebibtex::Bib>,
        pub settings: gio::Settings,
        pub file: RefCell<Option<gio::File>>,
        pub undo_entry: RefCell<Option<(cratebibtex::Entry, u32, adw::Toast)>>,
        pub force_close: Cell<bool>,

        #[template_child]
        pub main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub recent_files_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub recent_files_stack_page: TemplateChild<gtk::Widget>,
        #[template_child]
        pub empty_state_stack_page: TemplateChild<gtk::Widget>,
        #[template_child]
        pub recent_files_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub entry_list: TemplateChild<crate::EntryList>,
        #[template_child]
        pub entry_page: TemplateChild<crate::EntryPage>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub secondary_menu_button: TemplateChild<gtk::Widget>,
        #[template_child]
        pub nav_split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub recent_files_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub no_recent_files_status_page: TemplateChild<adw::StatusPage>,
    }

    impl Default for Window {
        fn default() -> Self {
            Self {
                settings: gio::Settings::new(APP_ID),
                bib: Default::default(),
                file: Default::default(),
                force_close: Default::default(),
                undo_entry: Default::default(),
                recent_files_status_page: TemplateChild::default(),
                no_recent_files_status_page: TemplateChild::default(),
                entry_list: TemplateChild::default(),
                recent_files_stack: TemplateChild::default(),
                main_stack: TemplateChild::default(),
                entry_page: TemplateChild::default(),
                search_bar: TemplateChild::default(),
                search_button: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                toast_overlay: TemplateChild::default(),
                secondary_menu_button: TemplateChild::default(),
                nav_split_view: TemplateChild::default(),
                recent_files_stack_page: TemplateChild::default(),
                empty_state_stack_page: TemplateChild::default(),
                recent_files_list: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action_async(
                "win.open",
                Some(glib::VariantTy::STRING),
                |window, _, param| {
                    // FIXME This clone is needed as long as the API requires a Option<&Value>.
                    let path: String = param.unwrap().get::<String>().unwrap();
                    async move {
                        if let Err(err) = window.open_action(path).await {
                            log::error!("Could not open file: {err}");
                        }
                    }
                },
            );
            klass.install_action_async("win.new_biblio", None, |window, _, _| async move {
                if let Err(err) = window.new_biblio_action().await {
                    log::error!("Could not create file: {err}");
                }
            });
            klass.install_action_async("win.new_entry", None, |window, _, _| async move {
                if let Err(err) = window.new_entry_dialog().await {
                    window.send_toast(&gettext("Could not add entry"));
                    log::error!("Could not spawn dialog: {err}");
                }
            });
            klass.install_action_async("win.save", None, |window, _, _| async move {
                if let Err(err) = window.save().await {
                    log::error!("Could not save bibliography: {err}");
                }
            });
            klass.install_action("win.delete_entry", None, move |obj, _, _| {
                if let Some(entry) = obj.imp().entry_page.entry() {
                    obj.delete_entry(&entry);
                }
            });
            klass.install_action("win.search-google-scholar", None, move |obj, _, _| {
                if let Err(err) = obj.search_google_schoolar_entry() {
                    log::error!("Could not search in Google Schoolar: {err}");
                }
            });
            klass.install_action("win.search-arxiv", None, move |obj, _, _| {
                if let Err(err) = obj.search_arxiv_entry() {
                    log::error!("Could not search in arXiv: {err}");
                }
            });
            klass.install_action("win.undo_delete", None, move |obj, _, _| {
                if let Some((entry, pos, _toast)) = obj.imp().undo_entry.take() {
                    obj.imp().bib.get().unwrap().add_entry_at_pos(entry, pos);
                }
            });
            klass.install_action("win.about", None, move |obj, _, _| {
                obj.show_about_dialog();
            });
            klass.install_action("win.search", None, move |obj, _, _| {
                obj.start_search_action();
            });
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl Window {
        #[template_callback]
        fn on_previous_match(&self) {
            self.entry_list.navigate_previous();
        }

        #[template_callback]
        fn on_next_match(&self) {
            self.entry_list.navigate_next();
        }

        #[template_callback]
        fn on_content_hidden(&self) {
            self.entry_list.hide_selection();
        }

        #[template_callback]
        fn on_search_mode_enabled_notify(
            &self,
            _pspec: &glib::ParamSpec,
            search_bar: &gtk::SearchBar,
        ) {
            if !search_bar.is_search_mode() {
                self.search_entry.set_text("");
            }
        }

        #[template_callback]
        fn on_collapsed_notify(
            &self,
            _pspec: glib::ParamSpec,
            nav_split_view: &adw::NavigationSplitView,
        ) {
            if nav_split_view.is_collapsed() {
                self.entry_list.hide_selection();
            } else {
                self.entry_list.show_selection();
                self.nav_split_view.set_show_content(true);
            }
        }

        #[template_callback]
        fn on_changed(&self, search_entry: &gtk::SearchEntry) {
            let query = search_entry.text();
            self.entry_list.filter(&query);
        }

        #[template_callback]
        fn on_entry_selected(&self, entry: Option<&cratebibtex::Entry>) {
            self.entry_page.set_entry(entry);
            self.secondary_menu_button.set_visible(entry.is_some());
            self.nav_split_view.set_show_content(true);
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Devel Profile
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
                self.recent_files_status_page.set_icon_name(Some(APP_ID));
                self.no_recent_files_status_page.set_icon_name(Some(APP_ID));
            }

            // Load latest window state
            obj.load_window_size();

            obj.populate_recent_files();
            self.settings.connect_changed(
                Some("recent-files"),
                glib::clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.populate_recent_files();
                    }
                ),
            );

            obj.action_set_enabled("win.new_entry", false);
            obj.action_set_enabled("win.save", false);
            obj.action_set_enabled("win.delete_entry", false);

            // Setup drag-n-drop
            let target = gtk::DropTarget::builder()
                .name("file-drop-target")
                .actions(gdk::DragAction::COPY | gdk::DragAction::MOVE)
                .formats(&gdk::ContentFormats::for_type(gdk::FileList::static_type()))
                .build();

            target.connect_drop(glib::clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    if let Ok(file_list) = value.get::<gdk::FileList>() {
                        if let Some(file) = file_list.files().first().cloned() {
                            glib::spawn_future_local(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    if let Err(err) = obj.open_wrapper(&file).await {
                                        log::error!("Could not open file: {err:?}")
                                    };
                                }
                            ));
                            return true;
                        }
                    }
                    false
                }
            ));

            obj.add_controller(target);
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {
        // Save window state on delete event
        fn close_request(&self) -> glib::Propagation {
            let window = self.obj();
            if let Err(err) = window.save_window_size() {
                log::warn!("Failed to save window state, {}", &err);
            }

            if let Some(bib) = self.bib.get() {
                if bib.modified() && !self.force_close.get() {
                    glib::spawn_future_local(glib::clone!(
                        #[weak]
                        window,
                        async move {
                            window.show_save_dialog().await;
                        }
                    ));

                    glib::Propagation::Stop
                } else {
                    self.parent_close_request()
                }
            } else {
                self.parent_close_request()
            }
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    async fn open_action(&self, path: String) -> anyhow::Result<()> {
        if path.is_empty() {
            let bib_filter = gtk::FileFilter::new();
            bib_filter.set_name(Some(&gettext("BibTeX Files")));
            bib_filter.add_mime_type("text/x-bibtex");
            bib_filter.add_pattern("*.bib");

            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&bib_filter);

            let dialog = gtk::FileDialog::new();
            dialog.set_filters(Some(&filters));

            let file = match dialog.open_future(Some(self)).await {
                Err(err) if err.matches(gtk::DialogError::Dismissed) => return Ok(()),
                res => res?,
            };
            self.open_wrapper(&file).await?;
        } else {
            let file = gio::File::for_path(&path);
            self.open_wrapper(&file).await?;
        }

        Ok(())
    }

    async fn new_biblio_action(&self) -> anyhow::Result<()> {
        let bib_filter = gtk::FileFilter::new();
        bib_filter.set_name(Some(&gettext("BibTeX Files")));
        bib_filter.add_mime_type("text/x-bibtex");
        bib_filter.add_pattern("*.bib");

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&bib_filter);

        let dialog = gtk::FileDialog::new();
        dialog.set_filters(Some(&filters));
        dialog.set_title(&gettext("Create New Bibliography"));
        dialog.set_accept_label(Some(&gettext("_Create")));
        // TRANSLATORS This a file name, .bib will be appended e.g.
        // bibliography.bib, please only use alphabetic characters.
        dialog.set_initial_name(Some(&(gettext("bibliography") + ".bib")));

        let file = match dialog.save_future(Some(self)).await {
            Err(err) if err.matches(gtk::DialogError::Dismissed) => return Ok(()),
            res => res?,
        };
        let stream = file
            .create_future(gio::FileCreateFlags::NONE, glib::Priority::default())
            .await?;
        stream.close_future(glib::Priority::default()).await?;
        self.open_wrapper(&file).await?;

        Ok(())
    }

    async fn open_wrapper(&self, file: &gio::File) -> anyhow::Result<()> {
        // TODO Maybe it makes more sense to open in the same window.
        if self.imp().bib.get().is_some() {
            let app = self
                .application()
                .and_downcast::<crate::Application>()
                .unwrap();
            let window = Window::new(&app);
            let group = gtk::WindowGroup::new();
            group.add_window(&window);
            window.open(file).await?;

            window.present();
        } else {
            self.open(file).await?;
        }

        Ok(())
    }

    pub async fn open(&self, file: &gio::File) -> anyhow::Result<()> {
        let slice = file.load_contents_future().await?.0;
        let data = std::str::from_utf8(&slice)?;
        let settings = &self.imp().settings;
        let bib = cratebibtex::Bib::default();
        bib.parse(data)?;
        if bib.n_items() != 0 {
            self.imp().entry_page.set_empty_status_page();
        }

        self.imp().entry_list.populate(&bib);
        self.imp().bib.set(bib).unwrap();
        self.imp().file.replace(Some(file.clone()));
        self.imp().search_bar.set_key_capture_widget(Some(self));
        self.imp()
            .main_stack
            .set_visible_child(&*self.imp().nav_split_view);

        self.action_set_enabled("win.new_entry", true);
        self.action_set_enabled("win.save", true);
        self.action_set_enabled("win.delete_entry", true);

        if let Some(path) = file.path() {
            if let Some(path) = path.to_str() {
                let mut recent_files = settings.strv("recent-files");

                if !recent_files.contains(path) {
                    recent_files.push(path.into());
                    settings.set_strv("recent-files", recent_files).unwrap();
                }
            }
        }

        Ok(())
    }

    fn populate_recent_files(&self) {
        let imp = self.imp();
        let model = gio::ListStore::new::<gio::File>();

        let recent_files = imp
            .settings
            .strv("recent-files")
            .into_iter()
            .rev()
            .map(gio::File::for_path)
            .filter(|file| file.query_exists(gio::Cancellable::NONE))
            .collect::<Vec<gio::File>>();
        model.splice(0, 0, &recent_files);

        imp.recent_files_list.bind_model(Some(&model), move |item| {
            let file = item.downcast_ref::<gio::File>().unwrap();
            recent_row(file).upcast()
        });

        if recent_files.is_empty() {
            imp.recent_files_stack
                .set_visible_child(&imp.empty_state_stack_page.get());
        } else {
            imp.recent_files_stack
                .set_visible_child(&imp.recent_files_stack_page.get());
        }
    }

    pub fn bib(&self) -> cratebibtex::Bib {
        self.imp().bib.get().cloned().unwrap()
    }

    pub fn send_toast(&self, text: &str) {
        let toast = adw::Toast::new(text);
        self.imp().toast_overlay.add_toast(toast);
    }

    pub async fn new_entry_dialog(&self) -> anyhow::Result<()> {
        let dialog = crate::NewEntryDialog::new();

        if let Some(data) = dialog.run(self).await {
            self.new_entry(data)
        } else {
            anyhow::bail!("Dialog error")
        }
    }

    pub fn new_entry(&self, data: String) -> anyhow::Result<()> {
        let entry = cratebibtex::Entry::from_bibtex(&data)?;
        let citation_key = entry.citation_key();
        if self.imp().bib.get().unwrap().key_exists(&citation_key) {
            self.send_toast(&gettext(
                "An entry with the given citation key already exists",
            ));
        } else {
            self.imp().search_bar.set_search_mode(false);
            let pos = self.imp().bib.get().unwrap().add_entry(entry);
            self.imp().entry_list.set_selected(pos);
        }

        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let file = self.imp().file.borrow().clone().unwrap();
        self.bib().save_to_file(&file).await?;

        self.send_toast(&gettext("Bibliography saved"));

        Ok(())
    }

    pub fn search_google_schoolar_entry(&self) -> anyhow::Result<()> {
        use anyhow::Context;

        let entry = self.imp().entry_page.entry().context("No entry selected")?;

        let author = entry.author();
        let title = entry.title();
        let p_author = format::texer(&author);
        let p_title = format::texer(&title);

        let uri = utils::build_google_schoolar_url(&p_author, &p_title)?;
        log::debug!("Trying to open {}", uri.as_str());

        let launcher = gtk::UriLauncher::new(uri.as_str());
        launcher.launch(Some(self), gio::Cancellable::NONE, move |res| {
            if let Err(err) = res {
                log::error!("Failed to launch uri {uri}: {err}");
            }
        });

        Ok(())
    }

    pub fn search_arxiv_entry(&self) -> anyhow::Result<()> {
        use anyhow::Context;

        let entry = self.imp().entry_page.entry().context("No entry selected")?;

        let author = entry.author();
        let title = entry.title();
        let p_author = format::texer(&author);
        let p_title = format::texer(&title);

        let uri = utils::build_arxiv_url(&p_author, &p_title)?;
        log::debug!("Trying to open {uri}");

        let launcher = gtk::UriLauncher::new(uri.as_str());
        launcher.launch(Some(self), gio::Cancellable::NONE, move |res| {
            if let Err(err) = res {
                log::error!("Failed to launch uri {uri}: {err}");
            }
        });

        Ok(())
    }

    pub fn delete_entry(&self, entry: &cratebibtex::Entry) {
        if let Some(pos) = self.bib().find_pos(entry) {
            let toast = adw::Toast::new(&gettext("Citation Deleted"));
            toast.set_action_name(Some("win.undo_delete"));
            toast.set_button_label(Some(&gettext("Undo")));
            toast.connect_dismissed(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    obj.imp().undo_entry.take();
                }
            ));

            if let Some((_, _, toast)) =
                self.imp()
                    .undo_entry
                    .replace(Some((entry.clone(), pos, toast.clone())))
            {
                toast.dismiss();
            };
            self.imp().entry_list.unselect();
            self.bib().remove_pos(pos);

            self.imp().toast_overlay.add_toast(toast);
            self.imp().nav_split_view.set_show_content(false);

            if self.imp().bib.get().unwrap().n_items() == 0 {
                self.imp().entry_page.set_no_items_placeholder_view();
            }
        };
    }

    fn start_search_action(&self) {
        let imp = self.imp();
        if imp.nav_split_view.is_collapsed() {
            imp.entry_list.hide_selection();
            imp.nav_split_view.set_show_content(false);
        }
        let search_mode = imp.search_bar.is_search_mode();
        imp.search_bar.set_search_mode(!search_mode);
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_icon(APP_ID)
            .application_name(gettext("Citations"))
            .license_type(gtk::License::Gpl30)
            .comments(gettext("Manage your bibliography"))
            .website("https://gitlab.gnome.org/World/citations/")
            .issue_url("https://gitlab.gnome.org/World/citations/-/issues/new")
            .version(VERSION)
            .translator_credits(gettext("translator-credits"))
            .developer_name("Maximiliano Sandoval")
            .developers(vec!["Maximiliano Sandoval <msandova@gnome.org>"])
            .designers(vec!["Tobias Bernard <tbernard@gnome.org>"])
            .build();

        dialog.present(Some(self));
    }

    async fn show_save_dialog(&self) {
        let dialog = adw::AlertDialog::new(
            // NOTE Dialog title which informs the user about unsaved changes.
            Some(&gettext("Unsaved Changes")),
            Some(&gettext(
                // NOTE Dialog subtitle which informs the user about unsaved changes more detailed.
                "Do you want to write all changes to the safe?",
            )),
        );

        dialog.add_response("discard", &gettext("_Quit Without Saving"));
        dialog.add_response("cancel", &gettext("_Don't Quit"));
        dialog.add_response("save", &gettext("_Save and Quit"));
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("save"));

        match dialog.choose_future(self).await.as_str() {
            "discard" => self.force_close(),
            "save" => {
                if let Err(err) = self.save().await {
                    log::error!("Could not save bib file: {err}");
                } else {
                    self.close();
                }
            }
            // There are more responses based on keybindings and random
            // interactions.
            _ => (),
        }
    }

    fn force_close(&self) {
        self.imp().force_close.set(true);
        self.close();
    }
}

fn recent_row(file: &gio::File) -> adw::ActionRow {
    let action_row = adw::ActionRow::new();

    if let Some(path) = file.path().and_then(|p| p.to_str().map(String::from)) {
        let action = format!("win.open::{path}");

        if !path.starts_with("/run/user") {
            action_row.set_subtitle(&path);
        }
        action_row.set_detailed_action_name(&action);
        action_row.set_activatable(true);

        let clear_button = gtk::Button::from_icon_name("edit-delete-symbolic");
        clear_button.set_valign(gtk::Align::Center);
        clear_button.connect_clicked(move |_button| {
            let settings = gio::Settings::new(APP_ID);
            crate::utils::remove_recent_file(&settings, &path);
        });
        clear_button.set_tooltip_text(Some(&gettext("Remove From Recent Files")));
        clear_button.add_css_class("flat");

        action_row.add_suffix(&clear_button);
    }

    if let Some(basename) = file.basename() {
        if let Some(basename) = basename.to_str() {
            action_row.set_title(basename);
        }
    }

    action_row
}
