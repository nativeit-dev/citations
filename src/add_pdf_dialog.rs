use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::CompositeTemplate;
use gtk::{gio, glib};
use std::cell::OnceCell;
use std::sync::LazyLock;

use crate::i18n::gettext_f;
use crate::utils;

mod imp {
    use super::*;

    use glib::subclass::Signal;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/World/Citations/ui/add_pdf_dialog.ui")]
    pub struct AddPdfDialog {
        pub citation_key: OnceCell<String>,

        #[template_child]
        pub download_button: TemplateChild<gtk::Widget>,
        #[template_child]
        pub pdf_url_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub error_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub error_box: TemplateChild<gtk::Widget>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddPdfDialog {
        const NAME: &'static str = "AddPdfDialog";
        type Type = super::AddPdfDialog;
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
    impl AddPdfDialog {
        #[template_callback]
        async fn set_from_file_row_activated(&self) {
            if let Err(err) = self.obj().set_pdf_from_file().await {
                log::error!("could not set pdf: {err}");
                let toast = adw::Toast::new(&gettext("Could not set PDF"));
                self.toast_overlay.add_toast(toast);
            }
        }

        #[template_callback]
        async fn on_entry_activated(&self) {
            self.download().await;
        }

        #[template_callback]
        async fn on_download_pdf_button_clicked(&self) {
            self.download().await;
        }

        async fn download(&self) {
            self.download_button.set_sensitive(false);
            if let Err(err) = self.obj().download_pdf_for_entry().await {
                log::error!("Could not download pdf: {err}");
                self.pdf_url_entry_row.add_css_class("error");
                let toast = adw::Toast::new(&gettext("Could not download PDF"));
                self.toast_overlay.add_toast(toast);
                self.download_button.set_sensitive(true);
                self.progress_bar.set_visible(false);
            } else {
                self.pdf_url_entry_row.remove_css_class("error");
                self.obj().emit_by_name::<()>("pdf-set", &[]);
                self.obj().close();
            }
        }
    }

    impl ObjectImpl for AddPdfDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> =
                LazyLock::new(|| vec![Signal::builder("pdf-set").run_last().build()]);
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            if crate::utils::bibliography_path().is_none() {
                let bibliography_path = crate::utils::bibliography_path_string();
                self.error_box.set_visible(true);
                self.error_label.set_label(&gettext_f(
                    // TRANSLATORS Do NOT translate {path}, it is a variable name
                    "Make sure that {path} exists",
                    &[("path", &bibliography_path)],
                ));
            }
        }
    }
    impl WidgetImpl for AddPdfDialog {}
    impl AdwDialogImpl for AddPdfDialog {}
}

glib::wrapper! {
    pub struct AddPdfDialog(ObjectSubclass<imp::AddPdfDialog>)
        @extends gtk::Widget, adw::Dialog, @implements gtk::Root;
}

impl Default for AddPdfDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl AddPdfDialog {
    pub fn new(citation_key: String) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().citation_key.set(citation_key).unwrap();

        dialog
    }

    async fn download_pdf_for_entry(&self) -> anyhow::Result<()> {
        let url_text = self.imp().pdf_url_entry_row.text();
        let url = url::Url::parse(&url_text)?;

        if let Some(path) = crate::utils::bibliography_path() {
            self.imp().progress_bar.set_visible(true);
            let duration = std::time::Duration::from_millis(100);
            let source = glib::timeout_add_local(
                duration,
                glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || {
                        obj.imp().progress_bar.pulse();

                        glib::ControlFlow::Continue
                    }
                ),
            );

            let res = self.download_pdf(url, &path).await;

            source.remove();

            res?;
        }

        Ok(())
    }

    async fn download_pdf(&self, url: url::Url, path: &std::path::Path) -> anyhow::Result<()> {
        let bytes = utils::get(url).await?;
        let bytes = glib::Bytes::from_owned(bytes);

        // Validate the document
        poppler::Document::from_bytes(&bytes, None)?;

        let citation_key = self.imp().citation_key.get().unwrap();

        let folder = gio::File::for_path(path);
        let pdf = folder.child(utils::pdf_filename(citation_key));

        pdf.replace_contents_future(bytes, None, false, gio::FileCreateFlags::NONE)
            .await
            .map_err(|err| err.1)?;

        Ok(())
    }

    async fn set_pdf_from_file(&self) -> anyhow::Result<()> {
        let citation_key = self.imp().citation_key.get().unwrap();

        if let Some(path) = crate::utils::bibliography_path() {
            // TODO Add djvu support, is it needed?
            let folder = gio::File::for_path(&path);

            let pdf_filter = gtk::FileFilter::new();
            pdf_filter.set_name(Some(&gettext("PDF Files")));
            pdf_filter.add_mime_type("application/pdf");

            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&pdf_filter);

            let dialog = gtk::FileDialog::new();
            dialog.set_filters(Some(&filters));
            dialog.set_title(&gettext("Select a PDF"));

            let window = self.root().and_downcast::<crate::Window>().unwrap();
            let file = match dialog.open_future(Some(&window)).await {
                Err(err) if err.matches(gtk::DialogError::Dismissed) => return Ok(()),
                res => res?,
            };
            let filename = utils::pdf_filename(citation_key);
            let dest = folder.child(&filename);

            if let Err(err) = std::fs::create_dir_all(&path) {
                log::error!("Could not create bibliography directory: {err}");
            }

            file.copy_future(&dest, gio::FileCopyFlags::NONE, glib::Priority::default())
                .0
                .await?;
            log::debug!("Copied {filename} to {}", path.to_str().unwrap());

            self.emit_by_name::<()>("pdf-set", &[]);
            self.close();
        }

        Ok(())
    }
}
