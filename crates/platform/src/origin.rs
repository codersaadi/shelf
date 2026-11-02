use crate::error::Result;
use std::path::Path;

/// xattr keys holding download origin (plan §12).
pub const ORIGIN_URL_KEYS: [&str; 2] = ["user.xdg.origin.url", "user.xdg.referrer.url"];
/// Browsers writing origin metadata as a plain app name.
pub const ORIGIN_APP_KEYS: [&str; 1] = ["user.xdg.origin.app"];
/// freedesktop shared tags: comma-separated (Nautilus-compatible).
pub const TAGS_KEY: &str = "user.xdg.tags";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Origin {
    pub url: Option<String>,
    pub referrer: Option<String>,
    pub app: Option<String>,
}

fn get_xattr(path: &Path, key: &str) -> Option<String> {
    xattr::get(path, key)
        .ok()
        .flatten()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn read_origin(path: &Path) -> Origin {
    Origin {
        url: get_xattr(path, ORIGIN_URL_KEYS[0]),
        referrer: get_xattr(path, ORIGIN_URL_KEYS[1]),
        app: get_xattr(path, ORIGIN_APP_KEYS[0]),
    }
}

pub fn read_tags_xattr(path: &Path) -> Vec<String> {
    get_xattr(path, TAGS_KEY)
        .map(|s| {
            s.split(',').map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).collect()
        })
        .unwrap_or_default()
}

pub fn write_tags_xattr(path: &Path, tags: &[String]) -> Result<()> {
    if tags.is_empty() {
        let _ = xattr::remove(path, TAGS_KEY);
        return Ok(());
    }
    xattr::set(path, TAGS_KEY, tags.join(",").as_bytes())?;
    Ok(())
}
