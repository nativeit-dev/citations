use gettextrs::gettext;
use regex::Regex;
use std::borrow::Cow;
use std::fmt;
use std::sync::LazyLock;

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "Format")]
#[repr(i32)]
pub enum Format {
    Plain = 0,
    Markdown = 1,
    LaTeX = 2,
}

impl Default for Format {
    fn default() -> Self {
        Self::Plain
    }
}

impl From<i32> for Format {
    fn from(i: i32) -> Self {
        match i {
            1 => Self::Markdown,
            2 => Self::LaTeX,
            _ => Self::Plain,
        }
    }
}

impl From<String> for Format {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Markdown" => Self::Markdown,
            "Latex" => Self::LaTeX,
            _ => Self::Plain,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Format {
    pub fn to_translatable_string(self) -> String {
        match self {
            // NOTE Plain as in Plain Text
            Format::Plain => gettext("Plain"),
            Format::Markdown => "Markdown".to_string(),
            Format::LaTeX => "LaTeX".to_string(),
        }
    }
}

pub(crate) fn format_plain(citation: &str) -> String {
    const MATCHES: &[char] = &['*', '_'];

    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\*\S[^\*]+\S\*|\*{2}[^\*]+\*{2}|_\S[^_]+\S_").unwrap());

    RE.replace_all(citation, |captures: &regex::Captures| {
        captures[0].trim_matches(MATCHES).to_string()
    })
    .to_string()
}

pub(crate) fn format_latex(citation: &str, key: &str) -> String {
    const MATCHES: &[char] = &['*', '_'];
    static IT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\*[^\*]+\*|_\S[^_]+\S_").unwrap());
    static BF_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\*{2}(?P<b>[^\*]+)\*{2}").unwrap());

    let formatted = BF_RE.replace_all(citation, r"\textbf{$b}").to_string();
    let formatted = IT_RE
        .replace_all(&formatted, |captures: &regex::Captures| {
            format!(r"\emph{{{}}}", captures[0].trim_matches(MATCHES))
        })
        .to_string();

    format!(r"\bibitem{{{key}}} {formatted}")
}

pub(crate) fn clear_citation(citation: &str) -> String {
    static CLEAR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\s*([,\.;:])(\s*[,\.;:]\s*)+|(\s)[\n\s]+").unwrap());
    static CLEAR_END_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*[,\.;:]\s*$").unwrap());
    static CLEAR_START_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\s*[,\.]\s*)+").unwrap());

    static NO_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"([\.,])\s*([Nn]o|[Vv]ol|pp)\.\s*[,\.]\s*").unwrap());

    static EMPTY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"("\s*"\s*|'\s*'\s*|\(\s*\)\s*|`\s*`\s*|\{\s*\}\s*)"#).unwrap()
    });

    let mut formatted = String::from(citation);

    // Remove "no.  ," and "no,   ."
    formatted = NO_RE.replace_all(&formatted, "$1 ").to_string();

    // Remove empty quotes, braces, etc
    while EMPTY_RE.is_match(&formatted) {
        formatted = EMPTY_RE.replace_all(&formatted, "").to_string();
    }

    // Flatten consecutive punctuation
    formatted = CLEAR_RE.replace_all(&formatted, "$1 ").to_string();
    // Turn last punctuation into a period
    formatted = CLEAR_END_RE.replace_all(&formatted, ".").to_string();
    // Remove punctuation at the start, e.g. " , some title" into "some title"
    formatted = CLEAR_START_RE.replace_all(&formatted, "").to_string();

    formatted
}

pub(crate) fn clear_markdown(citation: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(\*{2}\s*\*{2}|\*\s+\*|_\s*_)"#).unwrap());

    RE.replace_all(citation, "").to_string()
}

pub(crate) fn clear_non_markdown(citation: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"\*\s*\*\s*|\*{2}\s*\*{2}\s*|_\s*_\s*"#).unwrap());

    RE.replace_all(citation, "").to_string()
}

pub(crate) fn clear_latex(citation: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\\emph\{\s*\}|\\textbf\{\s*\}").unwrap());

    RE.replace_all(citation, "").to_string()
}

pub fn format_authors(authors: &str) -> String {
    let authors = authors.replace(" and ", " AND ").replace(" & ", " AND ");
    let mut vec_authors = authors
        .split(" AND ")
        .map(String::from)
        .collect::<Vec<String>>();
    if vec_authors.len() > 1 {
        if let Some(last_author) = vec_authors.last_mut() {
            let value = format!("and {}", last_author);
            *last_author = value;
        }
    }
    vec_authors.join(", ")
}

static COMBINING_MAP: &[(char, char)] = &[
    ('\'', '\u{301}'), // COMBINING ACUTE ACCENT
    ('`', '\u{300}'),  // COMBINING GRAVE
    ('~', '\u{303}'),  // COMBINING TILDE
    ('^', '\u{302}'),  // COMBINING CIRCUMFLEX ACCENT
    ('"', '\u{308}'),  // COMBINING DIAERESIS
    ('=', '\u{304}'),  // COMBINING MACRON
];

static COMBINING_MAP_ALPHA: &[(char, char)] = &[
    ('c', '\u{327}'), // COMBINING CEDILLA
    ('u', '\u{306}'), // COMBINING BREVE
    ('v', '\u{30C}'), // COMBINING CARON
    ('k', '\u{328}'), // COMBINING OGONEK
];

// See https://en.m.wikibooks.org/wiki/LaTeX/Special_Characters
static SYMBOLS_MAP: &[(char, char)] = &[('o', 'ø'), ('O', 'Ø'), ('l', 'ł'), ('L', 'Ł')];

pub fn push_math<T: Iterator<Item = char>>(chars: &mut T, output: &mut String) {
    match chars.next() {
        Some('$') => (),
        Some(c) => {
            push_non_space(output, c);
            push_math(chars, output);
        }
        _ => (),
    }
}

pub fn texer(input: &str) -> Cow<str> {
    let mut output = String::new();
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        match c {
            '$' => push_math(&mut chars, &mut output),
            '{' => {
                if let Some(c) = chars.next() {
                    let checkpoint = chars.clone();
                    match c {
                        '$' => push_math(&mut chars, &mut output),
                        '\\' => {
                            if let (Some(symbol), Some(paren)) = (chars.next(), chars.next()) {
                                if paren == '}' {
                                    if let Some((_, to)) =
                                        SYMBOLS_MAP.iter().find(|(from, _)| symbol == *from)
                                    {
                                        output.push(*to);
                                    } else {
                                        chars = checkpoint;
                                    }
                                } else if let Some((_, to)) =
                                    COMBINING_MAP.iter().find(|(from, _)| symbol == *from)
                                {
                                    output.push(paren);
                                    output.push(*to);
                                    if let Some('}') = chars.next() {
                                        continue;
                                    }
                                } else {
                                    output.push(symbol);
                                    push_non_space(&mut output, paren);
                                }
                            }
                        }
                        c => push_non_space(&mut output, c),
                    }
                }
            }
            '\\' => {
                if let Some(compose) = chars.next() {
                    let checkpoint = chars.clone();
                    if let Some((_, to)) = COMBINING_MAP_ALPHA
                        .iter()
                        .find(|(from, _)| compose == *from)
                    {
                        if let (Some('{'), Some(c), Some('}')) =
                            (chars.next(), chars.next(), chars.next())
                        {
                            output.push(c);
                            output.push(*to);
                        } else {
                            chars = checkpoint;
                            output.push(compose);
                        }
                    } else if let Some((_, to)) =
                        COMBINING_MAP.iter().find(|(from, _)| compose == *from)
                    {
                        if let Some(c) = chars.next() {
                            output.push(c);
                            output.push(*to);
                        } else {
                            output.push(compose);
                        }
                    } else {
                        output.push(compose);
                    }
                } else {
                    push_non_space(&mut output, c);
                }
            }
            '}' => (),
            c => push_non_space(&mut output, c),
        }
    }
    if !output.is_empty() {
        Cow::Owned(output)
    } else {
        Cow::Borrowed(input)
    }
}

fn push_non_space(output: &mut String, c: char) {
    if matches!(c, ' ' | '\t' | '\n') {
        if !output.ends_with(' ') {
            output.push(' ');
        }
    } else {
        output.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_citation() {
        assert_eq!(clear_citation("some,  text  2"), "some, text 2");

        assert_eq!(clear_citation("some, , , text"), "some, text");
        assert_eq!(clear_citation("some, ,  , ,  , text"), "some, text");
        assert_eq!(clear_citation("some, .  . ,  . text"), "some, text");
        assert_eq!(clear_citation("some. .  . ,  . , text"), "some. text");
        assert_eq!(clear_citation("some..,., ,, . , text"), "some. text");

        assert_eq!(clear_citation("some, no. , text"), "some, text");
        assert_eq!(clear_citation("some. No. . text"), "some. text");
        assert_eq!(clear_citation("some. no. , text"), "some. text");
        assert_eq!(clear_citation("some. vol. , text"), "some. text");
        assert_eq!(clear_citation("some. Vol. , text"), "some. text");

        assert_eq!(clear_citation("some. {} , text"), "some. text");
        assert_eq!(clear_citation("some. () , text"), "some. text");

        assert_eq!(clear_citation("some., ,"), "some.");
        assert_eq!(clear_citation(" , .some., ,"), "some.");

        assert_eq!(clear_citation(r#""""#), "");
        assert_eq!(clear_citation(r#"some, , "" , ,  , text"#), "some, text");
        assert_eq!(
            clear_citation(r#""some", , "" , ,  , text"#),
            r#""some", text"#
        );
    }

    #[test]
    fn test_format_authors() {
        assert_eq!(format_authors("x and y"), "x, and y");
        assert_eq!(format_authors("x and y and z"), "x, y, and z");
        assert_eq!(format_authors("x & y"), "x, and y");
        assert_eq!(format_authors("x AND y"), "x, and y");
        assert_eq!(format_authors("x and y and z"), "x, y, and z");
    }

    #[test]
    fn test_clear_latex() {
        assert_eq!(clear_latex(r"\textbf{}"), "");
        assert_eq!(clear_latex(r"\emph{}"), "");
    }

    #[test]
    fn test_clear_markdown() {
        assert_eq!(
            clear_markdown(r"asdas asd _asd_ *asd* ** asdasd **asd** **** __"),
            "asdas asd _asd_ *asd* asdasd **asd** "
        );
    }

    #[test]
    fn test_texer() {
        assert_eq!(texer(r"\'u"), "u\u{301}");
        assert_eq!(texer(r"\c{c}"), "c\u{0327}");
        assert_eq!(texer(r"\v{c}"), "c\u{30C}");
        assert_eq!(texer(r"{\o}a"), "øa");
        assert_eq!(texer(r"{\o}"), "ø");
        assert_eq!(texer(r#"{\"o}"#), "o\u{308}");
        assert_eq!(texer(r"a {some}"), "a some");
        assert_eq!(texer(r"a {\some}"), "a some");
        assert_eq!(texer(r"a {$some$}"), "a some");
        assert_eq!(texer(r"a  b"), r"a b");
        assert_eq!(texer(r"$a  b$"), r"a b");
        assert_eq!(
            texer(
                r"
                  "
            ),
            " "
        );
        assert_eq!(texer(r"{a  b}"), r"a b");
        assert_eq!(texer(r"a {$so\ ^\ast asda \me$}"), r"a so\ ^\ast asda \me");
        // assert_eq!(texer(r"{\ss}"), "ß");
    }
}
