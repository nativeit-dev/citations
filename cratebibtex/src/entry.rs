use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::LazyLock;

use gio::prelude::ListModelExt;
use glib::prelude::*;
use glib::subclass::prelude::*;
use regex::Regex;

use crate::format;
use crate::Format;

// TODO Make a better and more efficient parser
mod imp {
    use super::*;

    use glib::Properties;
    use std::cell::Cell;
    use std::cell::RefCell;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Entry)]
    pub struct Entry {
        pub tags: RefCell<HashMap<String, String>>,

        #[property(get, set = Self::set_other_entry_type, explicit_notify)]
        pub other_entry_type: RefCell<Option<String>>,
        #[property(type = String, get, set = Self::set_citation_key, explicit_notify)]
        pub citation_key: RefCell<Option<String>>,
        #[property(get, set = Self::set_entry_type, explicit_notify, builder(Default::default()))]
        pub entry_type: Cell<crate::EntryType>,

        // Custom properties
        #[property(get = |o: &Self| o.get("notes").unwrap_or_default(), set = |o: &Self, val| o.set("notes", val), explicit_notify)]
        notes: PhantomData<String>,
        #[property(get = |o: &Self| o.get("abstract").unwrap_or_default(), set = |o: &Self, val| o.set_abstract(val), explicit_notify)]
        x_abstract: PhantomData<String>,

        #[property(get = |o: &Self| o.get("author").unwrap_or_default(), set = |o: &Self, val| o.set("author", val), explicit_notify)]
        author: PhantomData<String>,
        #[property(get = |o: &Self| o.get("year").unwrap_or_default(), set = |o: &Self, val| o.set("year", val), explicit_notify)]
        year: PhantomData<String>,
        #[property(get = |o: &Self| o.get("title").unwrap_or_default(), set = |o: &Self, val| o.set("title", val), explicit_notify)]
        title: PhantomData<String>,
        #[property(get = |o: &Self| o.get("volume").unwrap_or_default(), set = |o: &Self, val| o.set("volume", val), explicit_notify)]
        volume: PhantomData<String>,
        #[property(get = |o: &Self| o.get("number").unwrap_or_default(), set = |o: &Self, val| o.set("number", val), explicit_notify)]
        number: PhantomData<String>,
        #[property(get = |o: &Self| o.get("pages").unwrap_or_default(), set = |o: &Self, val| o.set("pages", val), explicit_notify)]
        pages: PhantomData<String>,
        #[property(get = |o: &Self| o.get("publisher").unwrap_or_default(), set = |o: &Self, val| o.set("publisher", val), explicit_notify)]
        publisher: PhantomData<String>,
        #[property(get = |o: &Self| o.get("journal").unwrap_or_default(), set = |o: &Self, val| o.set("journal", val), explicit_notify)]
        journal: PhantomData<String>,
        #[property(get = |o: &Self| o.get("address").unwrap_or_default(), set = |o: &Self, val| o.set("address", val), explicit_notify)]
        address: PhantomData<String>,
        #[property(get = |o: &Self| o.get("howpublished").unwrap_or_default(), set = |o: &Self, val| o.set("howpublished", val), explicit_notify)]
        howpublished: PhantomData<String>,
        #[property(get = |o: &Self| o.get("note").unwrap_or_default(), set = |o: &Self, val| o.set("note", val), explicit_notify)]
        note: PhantomData<String>,
        #[property(get = |o: &Self| o.get("booktitle").unwrap_or_default(), set = |o: &Self, val| o.set("booktitle", val), explicit_notify)]
        booktitle: PhantomData<String>,
        #[property(get = |o: &Self| o.get("series").unwrap_or_default(), set = |o: &Self, val| o.set("series", val), explicit_notify)]
        series: PhantomData<String>,
        #[property(get = |o: &Self| o.get("archiveprefix").unwrap_or_default(), set = |o: &Self, val| o.set("archiveprefix", val), explicit_notify)]
        archiveprefix: PhantomData<String>,
        #[property(get = |o: &Self| o.get("eprint").unwrap_or_default(), set = |o: &Self, val| o.set("eprint", val), explicit_notify)]
        eprint: PhantomData<String>,
        #[property(get = |o: &Self| o.get("primaryclass").unwrap_or_default(), set = |o: &Self, val| o.set("primaryclass", val), explicit_notify)]
        primaryclass: PhantomData<String>,
        #[property(get = |o: &Self| o.get("month").unwrap_or_default(), set = |o: &Self, val| o.set("month", val), explicit_notify)]
        month: PhantomData<String>,
        #[property(get = |o: &Self| o.get("editor").unwrap_or_default(), set = |o: &Self, val| o.set("editor", val), explicit_notify)]
        editor: PhantomData<String>,
        #[property(get = |o: &Self| o.get("organization").unwrap_or_default(), set = |o: &Self, val| o.set("organization", val), explicit_notify)]
        organization: PhantomData<String>,
        #[property(get = |o: &Self| o.get("school").unwrap_or_default(), set = |o: &Self, val| o.set("school", val), explicit_notify)]
        school: PhantomData<String>,
        #[property(get = |o: &Self| o.get("institution").unwrap_or_default(), set = |o: &Self, val| o.set("institution", val), explicit_notify)]
        institution: PhantomData<String>,
        #[property(get = |o: &Self| o.get("doi").unwrap_or_default(), set = |o: &Self, val| o.set("doi", val), explicit_notify)]
        doi: PhantomData<String>,
        #[property(get = |o: &Self| o.get("url").unwrap_or_default(), set = |o: &Self, val| o.set("url", val), explicit_notify)]
        url: PhantomData<String>,
        #[property(get = |o: &Self| o.get("issn").unwrap_or_default(), set = |o: &Self, val| o.set("issn", val), explicit_notify)]
        issn: PhantomData<String>,
        #[property(get = |o: &Self| o.get("isbn").unwrap_or_default(), set = |o: &Self, val| o.set("isbn", val), explicit_notify)]
        isbn: PhantomData<String>,
    }

    impl Entry {
        fn get(&self, tag: &str) -> Option<String> {
            self.tags.borrow().get(tag).cloned()
        }

        fn set(&self, tag: &str, value: &str) {
            let obj = self.obj();
            let old_value = obj.insert_tag(tag, value);
            if old_value.as_deref() != Some(value)
                && obj.has_property(tag, Some(String::static_type()))
            {
                obj.notify(tag);
            }
        }

        fn set_abstract(&self, value: &str) {
            let old_value = self.obj().insert_tag("abstract", value);
            if old_value.as_deref() != Some(value) {
                self.obj().notify_x_abstract();
            }
        }

        fn set_entry_type(&self, entry_type: crate::EntryType) {
            if entry_type != self.entry_type.replace(entry_type) {
                self.obj().notify_entry_type();
            }
        }

        fn set_citation_key(&self, citation_key: &str) {
            let owned_citation_key = citation_key.to_string();
            if Some(citation_key)
                != self
                    .citation_key
                    .replace(Some(owned_citation_key))
                    .as_deref()
            {
                self.obj().notify_citation_key();
            }
        }

        fn set_other_entry_type(&self, other_type: &str) {
            self.other_entry_type.replace(Some(other_type.to_string()));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Entry {
        const NAME: &'static str = "Entry";
        type Type = super::Entry;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Entry {}
}

glib::wrapper! {
    pub struct Entry(ObjectSubclass<imp::Entry>);
}

impl Default for Entry {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl From<&nom_bibtex::Bibliography> for Entry {
    fn from(bib: &nom_bibtex::Bibliography) -> Self {
        let tags: HashMap<String, String> = bib.tags().clone();
        let entry = Self::default();
        entry.imp().tags.replace(tags);
        entry
            .imp()
            .citation_key
            .replace(Some(bib.citation_key().into()));

        let entry_type = bib.entry_type().into();
        if entry_type == crate::EntryType::Other {
            entry
                .imp()
                .other_entry_type
                .replace(Some(bib.entry_type().to_string()));
        }
        entry.imp().entry_type.replace(entry_type);

        entry
    }
}

impl Entry {
    pub fn from_bibtex(data: &str) -> anyhow::Result<Self> {
        let bib = crate::Bib::default();
        bib.parse(data)?;

        if let Some(entry) = bib.item(0) {
            let entry = entry.downcast::<Self>().unwrap();
            Ok(entry)
        } else {
            anyhow::bail!("Invalid BibTeX data");
        }
    }

    pub fn find_tag(&self, tag: &str) -> Option<String> {
        self.imp().tags.borrow().get(tag).cloned()
    }

    fn insert_tag(&self, tag: &str, value: &str) -> Option<String> {
        let mut hashmap = self.imp().tags.borrow_mut();

        hashmap.insert(tag.to_string(), value.into())
    }

    pub fn tags(&self) -> HashMap<String, String> {
        self.imp().tags.borrow().clone()
    }

    pub fn new(citation_key: &str, type_: crate::EntryType) -> Self {
        let entry = Self::default();
        entry
            .imp()
            .citation_key
            .replace(Some(citation_key.to_string()));
        entry.imp().entry_type.replace(type_);

        entry
    }

    pub fn fields(&self) -> Vec<String> {
        self.entry_type().fields()
    }

    pub fn format_citation(&self, citation: &str, format: Format) -> String {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%(?P<tag>[A-Z]+)%").unwrap());

        let mut formatted = String::from(citation);

        match format {
            Format::Plain => {
                formatted = format::format_plain(citation);
            }
            Format::Markdown => (),
            Format::LaTeX => {
                formatted = format::format_latex(citation, &self.citation_key());
            }
        }

        let name_prop = self.author();
        let p_name = match format {
            Format::LaTeX => Cow::Owned(name_prop),
            _ => format::texer(&name_prop),
        };
        formatted = formatted.replace("%AUTHOR%", &format::format_authors(&p_name));

        formatted = RE
            .replace_all(&formatted, |capture: &regex::Captures| {
                let tag = capture["tag"].to_lowercase();
                let opt_prop = self.find_tag(&tag).unwrap_or_default();
                let p_opt_prop = match format {
                    Format::LaTeX => Cow::Owned(opt_prop),
                    _ => {
                        if tag == "title" {
                            format::texer(&opt_prop)
                        } else {
                            Cow::Owned(opt_prop)
                        }
                    }
                };

                p_opt_prop.into_owned()
            })
            .into_owned();

        match format {
            Format::Markdown => format::clear_citation(&format::clear_markdown(&formatted)),
            Format::LaTeX => format::clear_citation(&format::clear_non_markdown(
                &format::clear_latex(&formatted),
            )),
            Format::Plain => format::clear_citation(&format::clear_non_markdown(&formatted)),
        }
    }

    pub fn serialize(&self) -> String {
        let citation_key = self.citation_key();
        let type_ = match self.entry_type() {
            crate::EntryType::Other => self
                .imp()
                .other_entry_type
                .borrow()
                .clone()
                .unwrap_or_else(|| "other".to_string()),
            s => s.to_string(),
        };
        let mut entry = format!("@{type_}{{{citation_key},\n");

        let lines = self
            .tags()
            .iter()
            .map(|(key, value)| format!("    {key} = {{{value}}},"))
            .collect::<Vec<String>>();
        entry += &lines.join("\n");

        entry += "\n}";

        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Bib;
    use gio::prelude::ListModelExt;

    const BIBFILE_DATA: &str = r#"
    @misc {my_citation_key,
        AUTHOR= "Some Author",
        title = "Some Title",
        year = "2020",
        volume = "5",
    }
"#;
    #[test]
    fn format() {
        let bib = Bib::default();
        bib.parse(BIBFILE_DATA).unwrap();

        let entry = bib.item(0).and_downcast::<Entry>().unwrap();

        let format = r#"%AUTHOR%: "%TITLE%" (%YEAR%)"#;
        let expected = r#"Some Author: "Some Title" (2020)"#;
        assert_eq!(entry.format_citation(format, Format::Plain), expected);

        let format = r#"%AUTHOR%: "_%TITLE%_" (**%VOLUME%**) (%YEAR%)"#;
        let expected_plain = r#"Some Author: "Some Title" (5) (2020)"#;
        let expected_markdown = r#"Some Author: "_Some Title_" (**5**) (2020)"#;
        let expected_latex =
            r#"\bibitem{my_citation_key} Some Author: "\emph{Some Title}" (\textbf{5}) (2020)"#;
        assert_eq!(entry.format_citation(format, Format::Plain), expected_plain);
        assert_eq!(
            entry.format_citation(format, Format::Markdown),
            expected_markdown
        );
        assert_eq!(entry.format_citation(format, Format::LaTeX), expected_latex);

        let format = r#"%AUTHOR%, %TITLE%, %VOLUME%, %JOURNAL%, %YEAR%"#;
        let expected = r#"Some Author, Some Title, 5, 2020"#;
        assert_eq!(entry.format_citation(format, Format::Plain), expected);

        let format = r#"%AUTHOR%, %TITLE%, %VOLUME%, "%JOURNAL%", %YEAR%"#;
        let expected = r#"Some Author, Some Title, 5, 2020"#;
        assert_eq!(entry.format_citation(format, Format::Plain), expected);

        let format = r#"%AUTHOR%, %TITLE%, %VOLUME%, (%JOURNAL%), "%FAKE%", %YEAR%"#;
        let expected = r#"Some Author, Some Title, 5, 2020"#;
        assert_eq!(entry.format_citation(format, Format::Plain), expected);

        let format = r#"%AUTHOR%, "%TITLE%", %VOLUME% {%JOURNAL%}, '%FAKE%',%YEAR%"#;
        let expected = r#"Some Author, "Some Title", 5, 2020"#;
        assert_eq!(entry.format_citation(format, Format::Plain), expected);

        let format = r#"%AUTHOR%, "*%TITLE%*", %VOLUME% *%JOURNAL%*, '%FAKE%',%YEAR%"#;
        let expected = r#"\bibitem{my_citation_key} Some Author, "\emph{Some Title}", 5, 2020"#;
        assert_eq!(entry.format_citation(format, Format::LaTeX), expected);

        let format = r#"%AUTHOR%, "*%TITLE%*", %VOLUME% (*%JOURNAL%*), '%FAKE%',%YEAR%"#;
        let expected = r#"\bibitem{my_citation_key} Some Author, "\emph{Some Title}", 5, 2020"#;
        assert_eq!(entry.format_citation(format, Format::LaTeX), expected);
    }
}
