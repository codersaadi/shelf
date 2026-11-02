use serde::Serialize;

/// Which family a chip belongs to; drives the leading glyph in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChipKind {
    Kind,
    Date,
    Size,
    Source,
    Place,
    Tag,
}

impl ChipKind {
    /// Icon name in the UI sprite (ui-ux.md §2.5 naming).
    pub fn glyph(self) -> &'static str {
        match self {
            ChipKind::Kind => "doc",
            ChipKind::Date => "clock",
            ChipKind::Size => "ruler",
            ChipKind::Source => "dl",
            ChipKind::Place => "folder",
            ChipKind::Tag => "tag",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Chip {
    pub kind: ChipKind,
    pub label: String,
    /// One-line explanation shown on hover (plan §2: explainable).
    pub description: String,
}

impl Chip {
    pub fn new(kind: ChipKind, label: impl Into<String>, description: impl Into<String>) -> Chip {
        Chip { kind, label: label.into(), description: description.into() }
    }
}
