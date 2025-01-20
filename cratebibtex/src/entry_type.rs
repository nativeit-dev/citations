use gettextrs::gettext;
use std::fmt;

// TODO Implement more types
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[repr(i32)]
#[enum_type(name = "EntryType")]
pub enum EntryType {
    #[default]
    Other = 0,
    Article = 1,
    Book = 2,
    Misc = 3,
    InProceedings = 4,
    Unpublished = 5,
    Online = 6,
    Booklet = 7,
    Conference = 8,
    InBook = 9,
    InCollection = 10,
    Manual = 11,
    MasterThesis = 12,
    PhdThesis = 13,
    Proceedings = 14,
    TechReport = 15,
}

impl From<i32> for EntryType {
    fn from(i: i32) -> Self {
        match i {
            1 => Self::Article,
            2 => Self::Book,
            3 => Self::Misc,
            4 => Self::InProceedings,
            5 => Self::Unpublished,
            6 => Self::Online,

            7 => Self::Booklet,
            8 => Self::Conference,
            9 => Self::InBook,
            10 => Self::InCollection,
            11 => Self::Manual,
            12 => Self::MasterThesis,
            13 => Self::PhdThesis,
            14 => Self::Proceedings,
            15 => Self::TechReport,
            _ => Self::default(),
        }
    }
}

impl From<&str> for EntryType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "article" => Self::Article,
            "book" => Self::Book,
            "misc" => Self::Misc,
            "inproceedings" => Self::InProceedings,
            "unpublished" => Self::Unpublished,
            "online" => Self::Online,

            "booklet" => Self::Booklet,
            "conference" => Self::Conference,
            "inbook" => Self::InBook,
            "incollection" => Self::InCollection,
            "manual" => Self::Manual,
            "masterthesis" => Self::MasterThesis,
            "phdthesis" => Self::PhdThesis,
            "proceedings" => Self::Proceedings,
            "techreport" => Self::TechReport,
            _ => Self::default(),
        }
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match &self {
            Self::Article => "article",
            Self::Book => "book",
            Self::Misc => "misc",
            Self::InProceedings => "inproceedings",
            Self::Unpublished => "unpublished",
            Self::Online => "online",
            Self::Other => "other",
            Self::Booklet => "booklet",
            Self::Conference => "conference",
            Self::InBook => "inbook",
            Self::InCollection => "incollection",
            Self::Manual => "manual",
            Self::MasterThesis => "masterthesis",
            Self::PhdThesis => "phdthesis",
            Self::Proceedings => "proceedings",
            Self::TechReport => "techreport",
        };
        write!(f, "{}", s)
    }
}

impl EntryType {
    pub fn uses_abstract(&self) -> bool {
        matches!(
            self,
            Self::Article
                | Self::InProceedings
                | Self::Unpublished
                | Self::Online
                | Self::InCollection
                | Self::MasterThesis
                | Self::PhdThesis
                | Self::Proceedings
        )
    }

    pub fn to_translatable_string(self) -> String {
        match self {
            Self::Article => gettext("Article"),
            Self::Book => gettext("Book"),
            Self::Misc => gettext("Misc"),
            // NOTE As in conference proceedings, see https://en.wikipedia.org/wiki/Conference_proceeding
            Self::InProceedings => gettext("In Proceedings"),
            Self::Unpublished => gettext("Unpublished"),
            // NOTE As in online source
            Self::Online => gettext("Online"),
            // NOTE As in other kind of source
            Self::Other => gettext("Other"),
            Self::Booklet => gettext("Booklet"),
            Self::Conference => gettext("Conference"),
            Self::InBook => gettext("In Book"),
            Self::InCollection => gettext("In Collection"),
            // NOTE As in instruction manual
            Self::Manual => gettext("Manual"),
            Self::MasterThesis => gettext("Master's Thesis"),
            Self::PhdThesis => gettext("PhD Thesis"),
            // NOTE As in conference proceedings, see https://en.wikipedia.org/wiki/Conference_proceeding
            Self::Proceedings => gettext("Proceedings"),
            Self::TechReport => gettext("Tech Report"),
        }
    }

    pub fn fields(&self) -> Vec<String> {
        match self {
            Self::Article => vec![
                "journal", "volume", "number", "pages", "month", "note", "issn", "eprint",
            ],
            Self::Book => vec![
                "publisher",
                "address",
                "editor",
                "volume",
                "number",
                "pages",
                "month",
                "note",
                "isbn",
                "eprint",
            ],
            Self::Misc => vec!["howpublished", "note", "eprint"],
            Self::InProceedings => vec![
                "booktitle",
                "series",
                "pages",
                "publisher",
                "address",
                "volume",
                "number",
                "isbn",
            ],
            Self::Unpublished => vec![],
            Self::Online => vec!["url", "archiveprefix", "eprint", "primaryclass"],
            Self::Other => vec![],
            Self::Booklet => vec!["howpublished", "month"],
            Self::Conference => vec!["booktitle", "series", "pages", "publisher", "address"],
            Self::InBook => vec!["booktitle", "address", "pages"],
            Self::InCollection => vec!["editor", "booktitle", "publisher", "address", "pages"],
            Self::Manual => vec!["organization", "address"],
            Self::MasterThesis => vec!["school", "address", "month"],
            Self::PhdThesis => vec!["school", "month", "address"],
            Self::Proceedings => vec!["editor", "series", "volume", "publisher", "address"],
            Self::TechReport => vec!["institution", "address", "number", "month"],
        }
        .into_iter()
        .map(String::from)
        .collect()
    }
}
