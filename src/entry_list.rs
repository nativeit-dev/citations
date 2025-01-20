// SPDX-License-Identifier: GPL-3.0-or-later
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};

mod imp {
    use super::*;

    use adw::subclass::prelude::*;
    use glib::subclass::Signal;
    use std::cell::RefCell;
    use std::sync::LazyLock;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/World/Citations/ui/entry_list.ui")]
    pub struct EntryList {
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub empty_list: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        pub selection_model: RefCell<crate::Selection>,
        pub filter_model: gtk::FilterListModel,
        pub filter: gtk::StringFilter,
        pub id: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntryList {
        const NAME: &'static str = "EntryList";
        type Type = super::EntryList;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl EntryList {
        #[template_callback]
        fn on_list_activate(&self, pos: u32) {
            self.selection_model.borrow().set_selected_position(pos);
        }
    }

    impl ObjectImpl for EntryList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
                vec![Signal::builder("entry-selected")
                    .param_types([Option::<cratebibtex::Entry>::static_type()])
                    .run_last()
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            let selection = crate::Selection::default();
            selection.set_model(Some(self.filter_model.upcast_ref()));
            selection.connect_notify_local(
                Some("selected-item"),
                glib::clone!(
                    #[weak]
                    obj,
                    move |model, _| {
                        let entry = model.selected_item().and_downcast::<cratebibtex::Entry>();
                        obj.emit_by_name::<()>("entry-selected", &[&entry]);
                    }
                ),
            );

            let factory = gtk::SignalListItemFactory::new();

            factory.connect_setup(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let row = crate::EntryListRow::default();
                item.set_child(Some(&row));
            });

            factory.connect_bind(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let child = item.child().unwrap();
                let row = child.downcast_ref::<crate::EntryListRow>().unwrap();

                let item = item.item().unwrap();
                row.set_entry(item.downcast_ref::<cratebibtex::Entry>().unwrap());
            });
            self.list_view.set_factory(Some(&factory));
            self.list_view.set_model(Some(&selection));

            self.selection_model.replace(selection);

            let search_expression = gtk::ClosureExpression::new::<String>(
                &[] as &[gtk::Expression],
                glib::closure!(|entry: cratebibtex::Entry| {
                    let author = entry.author();
                    let title = entry.title();
                    let year = entry.year();
                    let citation_key = entry.citation_key();

                    format!("{author} {title} {year} {citation_key}")
                }),
            );
            self.filter.set_expression(Some(&search_expression));
            self.filter
                .set_match_mode(gtk::StringFilterMatchMode::Substring);
            self.filter.set_ignore_case(true);

            self.filter_model.set_filter(Some(&self.filter));
        }
    }
    impl WidgetImpl for EntryList {}
    impl BinImpl for EntryList {}
}

glib::wrapper! {
    pub struct EntryList(ObjectSubclass<imp::EntryList>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for EntryList {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl EntryList {
    pub fn populate(&self, bib: &cratebibtex::Bib) {
        self.imp().filter_model.set_model(Some(bib));
        self.set_empty_view(bib.item(0).is_none());

        let id = bib.connect_items_changed(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |model, _, _, _| {
                obj.set_empty_view(model.item(0).is_none());
            }
        ));

        if let Some(old_id) = self.imp().id.replace(Some(id)) {
            bib.disconnect(old_id);
        }
    }

    pub fn filter(&self, query: &str) {
        self.imp().filter.set_search(Some(query));
    }

    fn set_empty_view(&self, is_empty: bool) {
        if is_empty {
            self.imp()
                .stack
                .set_visible_child(&self.imp().empty_list.get());
        } else {
            self.imp()
                .stack
                .set_visible_child(&self.imp().scrolled_window.get());
        }
    }

    pub fn set_selected(&self, pos: u32) {
        self.imp()
            .selection_model
            .borrow()
            .set_hide_selection(false);
        self.imp()
            .selection_model
            .borrow()
            .set_selected_position(pos);
        self.imp()
            .list_view
            .scroll_to(pos, gtk::ListScrollFlags::NONE, None::<gtk::ScrollInfo>);
    }

    pub fn hide_selection(&self) {
        self.imp().selection_model.borrow().set_hide_selection(true);
    }

    pub fn show_selection(&self) {
        self.imp()
            .selection_model
            .borrow()
            .set_hide_selection(false);
    }

    pub fn unselect(&self) {
        self.imp().selection_model.borrow().set_selected_item(None);
    }

    pub fn navigate_next(&self) {
        let imp = self.imp();

        let model = imp.selection_model.borrow();
        let scroll_flags = gtk::ListScrollFlags::NONE;

        let n_items = model.n_items();
        let pos = model.selected_item_pos();

        if pos == gtk::INVALID_LIST_POSITION && n_items > 0 {
            model.set_selected_position(0);
            imp.list_view
                .scroll_to(0, scroll_flags, None::<gtk::ScrollInfo>);
            return;
        }

        if pos + 1 < n_items {
            model.set_selected_position(pos + 1);
            imp.list_view
                .scroll_to(pos + 1, scroll_flags, None::<gtk::ScrollInfo>);
        }
    }

    pub fn navigate_previous(&self) {
        let imp = self.imp();

        let model = imp.selection_model.borrow();
        let scroll_flags = gtk::ListScrollFlags::NONE;

        let n_items = model.n_items();
        let pos = model.selected_item_pos();

        if pos == gtk::INVALID_LIST_POSITION && n_items > 0 {
            model.set_selected_position(n_items - 1);
            imp.list_view
                .scroll_to(n_items - 1, scroll_flags, None::<gtk::ScrollInfo>);
            return;
        }

        if pos > 0 {
            model.set_selected_position(pos - 1);
            imp.list_view
                .scroll_to(pos - 1, scroll_flags, None::<gtk::ScrollInfo>);
        }
    }
}
