// SPDX-License-Identifier: GPL-3.0-or-later
use cratebibtex::Format;
use gsv::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::CompositeTemplate;
use gettextrs::gettext;

const AMS_ARTICLE_TEMPLATE: &str =
    r#"%AUTHOR%, *%TITLE%*, %JOURNAL% **%VOLUME%** (%YEAR%), no. %NUMBER%, %PAGES%."#;
const MLA_ARTICLE_TEMPLATE: &str =
    r#"%AUTHOR%. "%TITLE%". *%JOURNAL%* %VOLUME%.%NUMBER% (%YEAR%): %PAGES%."#;
const APA_ARTICLE_TEMPLATE: &str =
    r#"%AUTHOR%. (%YEAR%). %TITLE%. *%JOURNAL%*, *%VOLUME%*(%NUMBER%), %PAGES%."#;
const CHICAGO_ARTICLE_TEMPLATE: &str =
    r#"%AUTHOR%. "%TITLE%". *%JOURNAL%* %VOLUME%, no. %NUMBER% (%YEAR%): %PAGES%."#;
const HARVAR_ARTICLE_TEMPLATE: &str =
    r#"%AUTHOR%, %YEAR%. %TITLE%. *%JOURNAL%*, *%VOLUME%*(%NUMBER%), pp.%PAGES%."#;
const VANCOUVER_ARTICLE_TEMPLATE: &str =
    r#"%AUTHOR%. %TITLE%. %JOURNAL%. %YEAR% %MONTH% %DAY%; %VOLUME%(%NUMBER%):%PAGES%."#;

const AMS_BOOK_TEMPLATE: &str = r#"%AUTHOR%, *%TITLE%*, %PUBLISHER%, %CITY%, %YEAR%."#;

const MLA_BOOK_TEMPLATE: &str = r#"%AUTHOR%. *%TITLE%*. Vol. %VOLUME%. %PUBLISHER%."#;
const APA_BOOK_TEMPLATE: &str = r#"%AUTHOR%. (%YEAR%). *%TITLE%*. (Vol. %VOLUME%). %PUBLISHER%."#;
const CHICAGO_BOOK_TEMPLATE: &str = r#"%AUTHOR%. *%TITLE%*. Vol. %VOLUME%. %PUBLISHER%, %YEAR%."#;
const HARVAR_BOOK_TEMPLATE: &str = r#"%AUTHOR%, %YEAR%. *%TITLE%* (Vol. %VOLUME%). %PUBLISHER%."#;
const VANCOUVER_BOOK_TEMPLATE: &str = r#"%AUTHOR%: %TITLE%. %PUBLISHER%; %YEAR% %MONTH% %DAY%."#;

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[repr(i32)]
#[enum_type(name = "Template")]
pub enum Template {
    Mla,
    Ams,
    Apa,
    Chicago,
    Harvard,
    Vancouver,
    Custom,
}

impl Default for Template {
    fn default() -> Self {
        Self::Mla
    }
}

impl Template {
    pub fn to_translatable_string(self) -> String {
        match self {
            Template::Mla => "MLA".to_string(),
            Template::Ams => "AMS".to_string(),
            Template::Apa => "APA".to_string(),
            // TODO Should this be translatable?
            Template::Chicago => gettext("Chicago"),
            Template::Harvard => gettext("Harvard"),
            Template::Vancouver => gettext("Vancouver"),
            // NOTE: Custom as in Custom Template.
            Template::Custom => gettext("Custom"),
        }
    }
}

impl From<i32> for Template {
    fn from(i: i32) -> Self {
        match i {
            0 => Self::Mla,
            1 => Self::Ams,
            2 => Self::Apa,
            3 => Self::Chicago,
            4 => Self::Harvard,
            5 => Self::Vancouver,
            6 => Self::Custom,
            _ => Self::default(),
        }
    }
}

mod imp {
    use super::*;

    use adw::subclass::prelude::*;
    use std::cell::{Cell, OnceCell, RefCell};
    use glib::Properties;
    use std::marker::PhantomData;

    #[derive(Debug, CompositeTemplate, Properties)]
    #[template(resource = "/org/gnome/World/Citations/ui/citation_preview.ui")]
    #[properties(wrapper_type = super::CitationPreview)]
    pub struct CitationPreview {
        #[template_child]
        pub source_view: TemplateChild<gsv::View>,
        #[template_child]
        pub text_view: TemplateChild<gtk::TextView>,
        #[template_child]
        pub drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub template_drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub template_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub help_source_view: TemplateChild<gtk::TextView>,

        #[property(get, set = Self::set_entry, explicit_notify)]
        entry: RefCell<Option<cratebibtex::Entry>>,
        #[property(get, set = Self::set_template_kind, explicit_notify, builder(Default::default()))]
        template_kind: Cell<Template>,
        #[property(get, set = Self::set_format_kind, explicit_notify, builder(Default::default()))]
        format_kind: Cell<cratebibtex::Format>,
        #[property(get = Self::template, explicit_notify)]
        template: PhantomData<String>,

        pub signals: OnceCell<glib::SignalGroup>,
        pub settings: gio::Settings,

    }

    impl Default for CitationPreview {
        fn default() -> Self {
            Self {
                source_view: TemplateChild::default(),
                text_view: TemplateChild::default(),
                drop_down: TemplateChild::default(),
                template_drop_down: TemplateChild::default(),
                template_revealer: TemplateChild::default(),
                help_source_view: TemplateChild::default(),
                entry: RefCell::default(),
                template_kind: Cell::default(),
                format_kind: Cell::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
                signals: Default::default(),
                template: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CitationPreview {
        const NAME: &'static str = "CitationPreview";
        type Type = super::CitationPreview;
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
    impl CitationPreview {
        fn set_entry(&self, entry: Option<&cratebibtex::Entry>) {
            let obj = self.obj();

            if entry == self.entry.replace(entry.cloned()).as_ref(){
             return;
            }

            let signals = self.signals.get().unwrap();
            signals.set_target(entry);

            if entry.is_some() {
             let template = if obj.use_book_format() {
             self.settings.string("custom-book-template")
                } else {
             self.settings.string("custom-article-template")
                };

             let buffer = self.text_view.buffer();
             buffer.set_text(&template);
            }

            obj.notify_template();
        }

        fn set_template_kind(&self, kind: Template) {
            if kind != self.template_kind.replace(kind) {
             self.obj().notify_template_kind();
             self.obj().notify_template();

             self.template_revealer
                    .set_reveal_child(kind == Template::Custom);
            }
        }

        fn set_format_kind(&self, kind: cratebibtex::Format) {
            if kind != self.format_kind.replace(kind) {
             let content = self.obj().template();
             self.obj().set_buffer(&content);
             self.obj().update_style_scheme();
             self.obj().notify_format_kind();
             self.obj().notify_template();
            }
        }

        fn template(&self) -> String {
            let obj = self.obj();
            match obj.template_kind() {
                Template::Custom => {
                    let buffer = self.text_view.buffer();
                    let (start, end) = buffer.bounds();
                    buffer.text(&start, &end, true).into()
                }
                Template::Ams => {
                    if obj.use_book_format() {
                        AMS_BOOK_TEMPLATE.into()
                    } else {
                        AMS_ARTICLE_TEMPLATE.into()
                    }
                }
                Template::Mla => {
                    if obj.use_book_format() {
                        MLA_BOOK_TEMPLATE.into()
                    } else {
                        MLA_ARTICLE_TEMPLATE.into()
                    }
                }
                Template::Apa => {
                    if obj.use_book_format() {
                        APA_BOOK_TEMPLATE.into()
                    } else {
                        APA_ARTICLE_TEMPLATE.into()
                    }
                }
                Template::Chicago => {
                    if obj.use_book_format() {
                        CHICAGO_BOOK_TEMPLATE.into()
                    } else {
                        CHICAGO_ARTICLE_TEMPLATE.into()
                    }
                }
                Template::Harvard => {
                    if obj.use_book_format() {
                        HARVAR_BOOK_TEMPLATE.into()
                    } else {
                        HARVAR_ARTICLE_TEMPLATE.into()
                    }
                }
                Template::Vancouver => {
                    if obj.use_book_format() {
                        VANCOUVER_BOOK_TEMPLATE.into()
                    } else {
                        VANCOUVER_ARTICLE_TEMPLATE.into()
                    }
                }
            }
        }

        #[template_callback]
        fn on_clicked(&self) {
            let buffer = self.source_view.buffer();
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, true);

            self.obj().clipboard().set_text(&text);

            let window = self
                .obj()
                .root()
                .and_downcast::<crate::Window>()
                .unwrap();
            window.send_toast(&gettext("Copied"));
        }

        #[template_callback]
        fn template_type_name(item: &adw::EnumListItem) -> String {
            let type_ = Template::from(item.value());
            if type_ == Template::Custom {
                gettext("Custom…")
            } else {
                type_.to_translatable_string()
            }
        }

        #[template_callback]
        fn format_type_name(item: &adw::EnumListItem) -> String {
            let type_ = Format::from(item.value());
            type_.to_translatable_string()
        }

        #[template_callback]
        fn on_template_notify(&self) {
            let template = self.obj().template();
            let selected = cratebibtex::Format::from(self.drop_down.selected() as i32);
            if let Some(entry) = &*self.entry.borrow() {
                let citation = entry.format_citation(&template, selected);
                self.source_view.buffer().set_text(&citation);
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for CitationPreview {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let signals = glib::SignalGroup::new::<cratebibtex::Entry>();
            self.signals.set(signals).unwrap();

            obj.update_style_scheme();
            let manager = adw::StyleManager::default();
            manager.connect_dark_notify(glib::clone!(#[weak] obj , move |_| {
                obj.update_style_scheme();
            }));

            self.source_view
                .buffer()
                .downcast::<gsv::Buffer>()
                .unwrap()
                .set_highlight_matching_brackets(false);

            self.help_source_view
                .buffer()
                .set_text(AMS_ARTICLE_TEMPLATE);

            self.text_view
                .buffer()
                .connect_changed(glib::clone!(#[weak] obj , move |buffer| {
                    let (start, end) = buffer.bounds();
                    let template = buffer.text(&start, &end, true);

                    if obj.use_book_format() {
                        obj.imp().settings.set_string("custom-book-template", &template).unwrap();
                    } else {
                        obj.imp().settings.set_string("custom-article-template", &template).unwrap();
                    }

                    obj.notify_template();
                }));

            self.settings
                .bind("template-kind", &self.template_drop_down.get(), "selected")
                .build();
            self.settings
                .bind("format-kind", &self.drop_down.get(), "selected")
                .build();

            self.template_drop_down
                .bind_property("selected", obj.as_ref(), "template-kind")
                .sync_create()
                .build();
            self.drop_down
                .bind_property("selected", obj.as_ref(), "format-kind")
                .sync_create()
                .build();

            let signals = self.signals.get().unwrap();
            signals.connect_notify_local(None, glib::clone!(#[weak] obj , move |_signal, _pspec| {
                obj.notify_template();
            }));
        }
    }
    impl WidgetImpl for CitationPreview {}
    impl BinImpl for CitationPreview {}
}

glib::wrapper! {
    pub struct CitationPreview(ObjectSubclass<imp::CitationPreview>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for CitationPreview {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl CitationPreview {
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

    fn set_buffer(&self, content: &str) {
        let buffer = match self.format_kind() {
            Format::Plain => gsv::Buffer::new(None),
            Format::Markdown => {
                let language_manager = gsv::LanguageManager::default();
                let lang = language_manager.language("markdown").unwrap();
                gsv::Buffer::with_language(&lang)
            }
            Format::LaTeX => {
                let language_manager = gsv::LanguageManager::default();
                let lang = language_manager.language("latex").unwrap();
                gsv::Buffer::with_language(&lang)
            }
        };

        buffer.set_highlight_syntax(true);
        buffer.set_highlight_matching_brackets(false);
        buffer.set_text(content);

        self.imp().source_view.set_buffer(Some(&buffer));
    }

    fn use_book_format(&self) -> bool {
        // TODO revisit this.
        if let Some(entry) = self.entry() {
            match entry.entry_type() {
                cratebibtex::EntryType::Other => false,
                cratebibtex::EntryType::Article => false,
                cratebibtex::EntryType::Book => true,
                cratebibtex::EntryType::Misc => false,
                cratebibtex::EntryType::InProceedings => true,
                cratebibtex::EntryType::Unpublished => false,
                cratebibtex::EntryType::Online => false,
                cratebibtex::EntryType::Booklet => true,
                cratebibtex::EntryType::Conference => false,
                cratebibtex::EntryType::InBook => true,
                cratebibtex::EntryType::InCollection => true,
                cratebibtex::EntryType::Manual => false,
                cratebibtex::EntryType::MasterThesis => false,
                cratebibtex::EntryType::PhdThesis => false,
                cratebibtex::EntryType::Proceedings => false,
                cratebibtex::EntryType::TechReport => false,
            }
        } else {
            false
        }
    }
}
