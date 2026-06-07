use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct DocumentId(pub usize);

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Field {
    Title,
    Heading,
    Body,
    Tag,
    Path,
}

impl Field {
    pub fn weight(self) -> f32 {
        match self {
            Self::Title => 4.0,
            Self::Heading => 2.5,
            Self::Tag => 2.0,
            Self::Path => 1.5,
            Self::Body => 1.0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum LinkKind {
    Wiki,
    Markdown,
    External,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Link {
    pub raw: String,
    pub target: String,
    pub text: Option<String>,
    pub line: usize,
    pub kind: LinkKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub id: DocumentId,
    pub path: PathBuf,
    pub title: Option<String>,
    pub headings: Vec<Heading>,
    pub body: String,
    pub lines: Vec<String>,
    pub links: Vec<Link>,
    pub tags: Vec<String>,
}

impl Document {
    pub fn display_title(&self) -> String {
        self.title
            .clone()
            .or_else(|| {
                self.path
                    .file_stem()
                    .map(|value| value.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| self.path.display().to_string())
    }
}
