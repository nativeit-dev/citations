use anyhow::Result;
use gio::prelude::*;
use gio::subclass::prelude::*;
use std::cell::RefCell;

use crate::Entry;

mod imp {
    use super::*;

    use glib::Properties;
    use std::cell::Cell;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Bib)]
    pub struct Bib {
        pub inner: RefCell<nom_bibtex::Bibtex>,
        pub entries: RefCell<Vec<Entry>>,
        pub n_items: Cell<u32>,

        #[property(get, set = Self::set_modified, explicit_notify)]
        modified: Cell<bool>,
    }

    impl Bib {
        pub fn set_modified(&self, modified: bool) {
            if modified != self.modified.replace(modified) {
                self.obj().notify_modified();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Bib {
        const NAME: &'static str = "Bib";
        type Type = super::Bib;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for Bib {}

    impl ListModelImpl for Bib {
        fn item_type(&self) -> glib::Type {
            Entry::static_type()
        }

        fn n_items(&self) -> u32 {
            self.n_items.get()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.entries
                .borrow()
                .get(position as usize)
                .cloned()
                .and_upcast()
        }
    }
}

glib::wrapper! {
    pub struct Bib(ObjectSubclass<imp::Bib>) @implements gio::ListModel;
}

impl Default for Bib {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Bib {
    pub fn parse(&self, data: &str) -> Result<()> {
        let bibtex = nom_bibtex::Bibtex::parse(data)?;
        let entries = bibtex
            .bibliographies()
            .iter()
            .map(From::from)
            .collect::<Vec<Entry>>();
        let n_items = entries.len() as u32;

        for entry in entries.iter() {
            entry.connect_notify_local(
                None,
                glib::clone!(
                    #[weak(rename_to = bib)]
                    self,
                    #[weak]
                    entry,
                    move |_entry, pspec| {
                        log::debug!(
                            "Entry {}'s property {} was modified",
                            entry.citation_key(),
                            pspec.name()
                        );
                        bib.set_modified(true);
                    }
                ),
            );
        }

        self.imp().inner.replace(bibtex);
        let old_entries = self.imp().entries.replace(entries);

        self.imp().n_items.set(n_items);
        self.items_changed(0, old_entries.len() as u32, n_items);

        Ok(())
    }

    pub fn serialize(&self) -> String {
        let mut entries = vec![];
        for entry in self.imp().entries.borrow().iter() {
            entries.push(entry.serialize())
        }
        entries.join("\n\n")
    }

    pub fn add_entry(&self, entry: Entry) -> u32 {
        {
            // We create a block to free the borrow asap.
            let mut entries = self.imp().entries.borrow_mut();
            entry.connect_notify_local(
                None,
                glib::clone!(
                    #[weak(rename_to = bib)]
                    self,
                    #[weak]
                    entry,
                    move |_entry, pspec| {
                        log::debug!(
                            "Entry {} property {} was modified",
                            entry.citation_key(),
                            pspec.name()
                        );
                        bib.set_modified(true);
                    }
                ),
            );
            entries.push(entry);
        };
        let n_items = self.imp().n_items.get();
        self.imp().n_items.set(self.imp().n_items.get() + 1);
        self.items_changed(n_items, 0, 1);
        self.set_modified(true);
        n_items
    }

    pub fn add_entry_at_pos(&self, entry: Entry, pos: u32) {
        let mut pos = pos;
        {
            // We create a block to free the borrow asap.
            let mut entries = self.imp().entries.borrow_mut();
            pos = pos.min(entries.len() as u32);
            entry.connect_notify_local(
                None,
                glib::clone!(
                    #[weak(rename_to = bib)]
                    self,
                    #[weak]
                    entry,
                    move |_entry, pspec| {
                        log::debug!(
                            "Entry {} property {} was modified",
                            entry.citation_key(),
                            pspec.name()
                        );
                        bib.set_modified(true);
                    }
                ),
            );
            entries.insert(pos as usize, entry);
        };
        self.imp().n_items.set(self.imp().n_items.get() + 1);
        self.items_changed(pos, 0, 1);
        self.set_modified(true);
    }

    pub fn remove_entry(&self, citation_key: &str) {
        let mut entries = self.imp().entries.borrow_mut();
        if let Some((idx, _)) = entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.citation_key() == citation_key)
        {
            entries.remove(idx);
            self.imp().n_items.set(self.imp().n_items.get() - 1);
            self.items_changed(idx as u32, 1, 0);
        }
        self.set_modified(true);
    }

    pub fn remove_pos(&self, pos: u32) {
        self.imp().entries.borrow_mut().remove(pos as usize);
        self.imp().n_items.set(self.imp().n_items.get() - 1);
        self.items_changed(pos, 1, 0);
        self.set_modified(true);
    }

    pub fn find_pos(&self, entry: &crate::Entry) -> Option<u32> {
        for (pos, e) in self.imp().entries.borrow().iter().enumerate() {
            if e.citation_key() == entry.citation_key() {
                return Some(pos as u32);
            }
        }
        None
    }

    pub fn key_exists(&self, citation_key: &str) -> bool {
        self.imp()
            .entries
            .borrow()
            .iter()
            .any(|entry| entry.citation_key() == citation_key)
    }

    pub async fn save_to_file(&self, file: &gio::File) -> anyhow::Result<()> {
        let content = self.serialize();
        file.replace_contents_future(content, None, true, gio::FileCreateFlags::NONE)
            .await
            .map_err(|err| err.1)?;
        self.set_modified(false);
        if let Some(path) = file.path() {
            log::debug!("Bibliography {} was successfully saved", path.display());
        }

        Ok(())
    }
}
