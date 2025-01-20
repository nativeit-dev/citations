// SPDX-License-Identifier: GPL-3.0-or-later
use gettextrs::gettext;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};
use gtk::{prelude::*, CompositeTemplate};

mod imp {
    use super::*;

    use glib::Properties;
    use std::cell::RefCell;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/org/gnome/World/Citations/ui/entry_list_row.ui")]
    #[properties(wrapper_type = super::EntryListRow)]
    pub struct EntryListRow {
        #[template_child]
        pub child: TemplateChild<gtk::Widget>,
        #[template_child]
        pub author: TemplateChild<gtk::Label>,
        #[template_child]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        #[property(get, set)]
        entry: RefCell<Option<cratebibtex::Entry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntryListRow {
        const NAME: &'static str = "EntryListRow";
        type Type = super::EntryListRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_layout_manager_type::<gtk::BinLayout>();

            // Action used only for the extra menu
            klass.install_action_async("entry-row.open-pdf", None, |row, _, _| async move {
                let citation_key = row.entry().unwrap().citation_key();
                let window = row.root().and_downcast::<crate::Window>().unwrap();

                if let Err(err) = crate::utils::open_pdf(&citation_key, &window).await {
                    log::error!("Could not open pdf: {err}");
                }
            });

            klass.install_action("entry-row.delete", None, move |row, _, _| {
                let window = row.root().and_downcast::<crate::Window>().unwrap();
                window.delete_entry(&row.entry().unwrap());
            });

            klass.install_action("entry-row.copy-key", None, move |row, _, _| {
                let citation_key = row.entry().unwrap().citation_key();
                row.clipboard().set_text(&citation_key);

                let window = row.root().and_downcast::<crate::Window>().unwrap();
                window.send_toast(&gettext("Copied"));
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl EntryListRow {
        #[template_callback]
        fn pretty_author(&self, author: &str) -> String {
            let author = cratebibtex::format::texer(author);
            if author.is_empty() {
                let placeholder = gettext("Author not set");
                format!("<i>{placeholder}</i>")
            } else {
                author.into_owned()
            }
        }

        #[template_callback]
        fn pretty_title(&self, title: &str) -> String {
            let title = cratebibtex::format::texer(title);
            if title.is_empty() {
                let placeholder = gettext("Title not set");
                format!("<i>{placeholder}</i>")
            } else {
                title.into_owned()
            }
        }

        #[template_callback]
        fn menu_gesture_clicked(obj: &super::EntryListRow, _n_press: i32, x: f64, y: f64) {
            let has_pdf = crate::utils::has_pdf(&obj.entry().unwrap().citation_key());
            obj.action_set_enabled("entry-row.open-pdf", has_pdf);

            if x > -1.0 && y > -1.0 {
                let rectangle = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                obj.imp().popover.set_pointing_to(Some(&rectangle));
            } else {
                obj.imp().popover.set_pointing_to(None);
            }
            obj.imp().popover.popup();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntryListRow {
        fn dispose(&self) {
            self.dispose_template();
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            if obj.direction() == gtk::TextDirection::Rtl {
                self.popover.set_halign(gtk::Align::End);
            } else {
                self.popover.set_halign(gtk::Align::Start);
            }
        }
    }
    impl WidgetImpl for EntryListRow {}
}

glib::wrapper! {
    pub struct EntryListRow(ObjectSubclass<imp::EntryListRow>)
        @extends gtk::Widget;
}

impl Default for EntryListRow {
    fn default() -> Self {
        glib::Object::new()
    }
}
