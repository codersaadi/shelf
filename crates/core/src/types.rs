use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Image,
    Video,
    Music,
    Document,
    Archive,
    App,
    Other,
}

impl FileKind {
    pub const ALL: [FileKind; 6] = [
        FileKind::Image,
        FileKind::Video,
        FileKind::Music,
        FileKind::Document,
        FileKind::Archive,
        FileKind::App,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FileKind::Image => "image",
            FileKind::Video => "video",
            FileKind::Music => "music",
            FileKind::Document => "document",
            FileKind::Archive => "archive",
            FileKind::App => "app",
            FileKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub kind: FileKind,
    pub size: u64,
    pub mtime: i64,
    /// Filesystem creation time when available, else mtime. Drives "Just added"
    /// and Recent; index insertion time would make a first scan look like every
    /// file was downloaded at once.
    pub created: i64,
    pub duration_ms: Option<u64>,
    pub inode: Option<u64>,
    pub is_screenshot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub id: i64,
    pub path: String,
    pub parent: String,
    pub name: String,
    pub ext: String,
    pub kind: FileKind,
    pub size: u64,
    pub mtime: i64,
    pub added_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub is_screenshot: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_host: Option<String>,
    pub in_trash: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trash_path: Option<String>,
    /// Times the user opened this file from within Shelf (frecency signal).
    #[serde(skip_serializing_if = "is_zero")]
    pub opens: u64,
    /// Open count decayed by age; used only for ordering.
    #[serde(skip_serializing)]
    pub frecency: i64,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

/// SQL-shaped query. `shelf-query`'s `Filter` maps onto this; views and sorts are UI concepts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub kinds: Vec<FileKind>,
    /// Extension filters without the dot, e.g. `pdf`.
    pub exts: Vec<String>,
    /// Inclusive unix-second bounds on file modification time.
    pub mtime_from: Option<i64>,
    pub mtime_to: Option<i64>,
    /// Inclusive byte-size bounds.
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    /// Substring match against origin url, app, or host; any may match.
    pub sources: Vec<String>,
    /// Screenshot filter: `Some(true)` only screenshots, `Some(false)` only non-screenshots.
    pub screenshot: Option<bool>,
    /// Exact file names (AND semantics).
    pub names: Vec<String>,
    /// Tags that must all be present (AND semantics).
    pub tags: Vec<String>,
    /// Hide source-code extensions from "recently added" views.
    pub exclude_code: bool,
    /// Substring terms, all of which must match (FTS5 over filename).
    pub text: Vec<String>,
    /// Path prefix, e.g. `~/Pictures/Screenshots`.
    pub under_path: Option<String>,
    /// Limit a folder browse to files whose parent is exactly `under_path`.
    /// Free-text searches keep recursive path-prefix semantics.
    pub direct_children: bool,
    pub in_trash: bool,
    pub sort: Sort,
    pub limit: u32,
    /// Opaque cursor from a previous page.
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    #[default]
    Modified,
    Added,
    Size,
    Name,
    Kind,
    Frecency,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub items: Vec<FileMeta>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Kind(FileKind),
    Downloads,
    Screenshots,
    Trash,
    Recent,
}

#[derive(Debug, Clone, Serialize)]
pub struct KindStat {
    pub kind: FileKind,
    pub count: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub id: i64,
    pub action: String,
    /// JSON array of { path, trash_path, size } entries moved.
    pub paths: String,
    pub detail: Option<String>,
    pub created: i64,
    pub undone: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedSearch {
    pub id: i64,
    pub name: String,
    pub query: String,
    pub created: i64,
}
