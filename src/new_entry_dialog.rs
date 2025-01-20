// SPDX-License-Identifier: GPL-3.0-or-later
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gsv::prelude::*;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::utils;

const SAMPLE_ENTRY: &str = r"@book{lamport1985i1,
  title={\LaTeX: A Document Preparation System},
  author={Lamport, Leslie},
  year={1991},
  publisher={Addison-Wesley}
}";

mod imp {
    use super::*;

    use std::cell::RefCell;
    use std::sync::Once;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/org/gnome/World/Citations/ui/new_entry_dialog.ui")]
    pub struct NewEntryDialog {
        #[template_child]
        pub combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub source_view: TemplateChild<gsv::View>,
        #[template_child]
        pub doi_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub view_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub create_button: TemplateChild<gtk::Button>,

        pub sender: RefCell<Option<futures_channel::oneshot::Sender<Option<String>>>>,
        pub remove_text: Once,
    }

    impl Default for NewEntryDialog {
        fn default() -> Self {
            Self {
                combo_row: TemplateChild::default(),
                entry_row: TemplateChild::default(),
                source_view: TemplateChild::default(),
                doi_row: TemplateChild::default(),
                view_stack: TemplateChild::default(),
                create_button: TemplateChild::default(),
                sender: RefCell::default(),
                remove_text: Once::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NewEntryDialog {
        const NAME: &'static str = "NewEntryDialog";
        type Type = super::NewEntryDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl NewEntryDialog {
        #[template_callback]
        fn entry_type_name(item: &adw::EnumListItem) -> String {
            let type_ = cratebibtex::EntryType::from(item.value());
            type_.to_translatable_string()
        }

        #[template_callback]
        fn on_cancel_clicked(&self) {
            let sender = self.sender.take().unwrap();
            sender.send(None).unwrap();
            self.obj().close();
        }

        #[template_callback]
        fn on_entry_notify_text(&self) {
            self.obj().update_create_button();
        }

        #[template_callback]
        fn on_doi_notify_text(&self) {
            self.obj().update_create_button();
        }

        #[template_callback]
        fn on_stack_visible_child_notify(&self) {
            self.obj().update_create_button();
        }

        #[template_callback]
        async fn on_create_clicked(&self) {
            let sender = self.sender.take().unwrap();

            let data = match &self.view_stack.visible_child_name().as_deref() {
                Some("new") => {
                    let citation_key = self.entry_row.text().to_string();
                    let type_ =
                        cratebibtex::EntryType::from(self.combo_row.selected() as i32).to_string();
                    Some(format!("@{type_}{{{citation_key},}}"))
                }
                Some("from_bibtex") => {
                    let buffer = self.source_view.buffer();
                    let (start, end) = buffer.bounds();
                    Some(buffer.text(&start, &end, true).into())
                }
                Some("from_doi") => utils::bib_from_doi(&self.doi_row.text())
                    .await
                    .inspect_err(|err| log::error!("Could not get bib from doi: {err}"))
                    .ok(),
                _ => unreachable!(),
            };
            sender.send(data).unwrap();

            self.obj().close();
        }
    }

    impl ObjectImpl for NewEntryDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let language_manager = gsv::LanguageManager::default();
            let lang = language_manager.language("bibtex").unwrap();
            let buffer = gsv::Buffer::with_language(&lang);

            buffer.set_highlight_syntax(true);
            buffer.set_highlight_matching_brackets(false);
            self.source_view.set_buffer(Some(&buffer));

            buffer.connect_changed(glib::clone!(
                #[weak]
                obj,
                move |_buffer| obj.update_create_button()
            ));

            obj.update_style_scheme();
            let manager = adw::StyleManager::default();
            manager.connect_dark_notify(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_style_scheme();
                }
            ));

            let instructions = gettext("Insert a BibTeX citation here");
            buffer.set_text(&format!("{instructions}\n\n{SAMPLE_ENTRY}"));

            let controller = gtk::EventControllerFocus::new();
            controller.connect_enter(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    obj.imp().remove_text.call_once(glib::clone!(
                        #[weak]
                        obj,
                        move || {
                            let buffer = obj.imp().source_view.buffer();
                            buffer.set_text("");
                        }
                    ));
                }
            ));
            self.source_view.get().add_controller(controller);
        }
    }

    impl WidgetImpl for NewEntryDialog {}
    impl AdwDialogImpl for NewEntryDialog {}
}

glib::wrapper! {
    pub struct NewEntryDialog(ObjectSubclass<imp::NewEntryDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl Default for NewEntryDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl NewEntryDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub async fn run<W: glib::object::IsA<gtk::Widget>>(&self, window: &W) -> Option<String> {
        let (sender, receiver) = futures_channel::oneshot::channel();
        self.imp().sender.replace(Some(sender));

        self.present(Some(window));

        receiver.await.unwrap_or_default()
    }

    fn update_style_scheme(&self) {
        let manager = adw::StyleManager::default();
        let scheme_name = if manager.is_dark() {
            "Adwaita-dark"
        } else {
            "Adwaita"
        };
        let scheme = gsv::StyleSchemeManager::default()
            .scheme(scheme_name)
            .unwrap();

        let buffer = self
            .imp()
            .source_view
            .buffer()
            .downcast::<gsv::Buffer>()
            .unwrap();
        buffer.set_style_scheme(Some(&scheme));
    }

    fn update_create_button(&self) {
        let imp = self.imp();

        let is_text_empty = match &imp.view_stack.visible_child_name().as_deref() {
            Some("new") => imp.entry_row.text().is_empty(),
            Some("from_bibtex") => {
                let buffer = imp.source_view.buffer();
                let (start, end) = buffer.bounds();
                let text = buffer.text(&start, &end, true);

                text.is_empty()
            }
            Some("from_doi") => imp.doi_row.text().is_empty(),
            _ => unreachable!(),
        };

        imp.create_button.set_sensitive(!is_text_empty);
    }
}
