// SPDX-License-Identifier: GPL-3.0-or-later
mod application;
#[rustfmt::skip]
mod citation_preview;
mod add_pdf_dialog;
mod config;
mod entry_list;
mod entry_list_row;
mod entry_page;
mod i18n;
mod new_entry_dialog;
mod selection;
mod utils;
mod window;

use gettextrs::{gettext, LocaleCategory};
use gtk::{gio, glib, prelude::*};

pub use add_pdf_dialog::AddPdfDialog;
pub use citation_preview::CitationPreview;
pub use entry_list::EntryList;
pub use entry_list_row::EntryListRow;
pub use entry_page::EntryPage;
pub use new_entry_dialog::NewEntryDialog;
use selection::Selection;
pub use window::Window;

use self::application::Application;
use self::config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

fn main() -> glib::ExitCode {
    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");
    cratebibtex::init(GETTEXT_PACKAGE, LOCALEDIR);

    // NOTE: This is the app name
    glib::set_application_name(&gettext("Citations"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = Application::default();

    // Define gobject types
    EntryList::static_type();
    EntryListRow::static_type();
    EntryPage::static_type();
    CitationPreview::static_type();
    Window::static_type();
    NewEntryDialog::static_type();
    Selection::static_type();

    app.run()
}
