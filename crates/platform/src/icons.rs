//! Native application icon lookup: desktop entries, icon themes and adjacent
//! AppImage artwork. This is deliberately read-only and process-local.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default)]
struct DesktopEntry {
    id: String,
    name: String,
    icon: String,
    exec: String,
    try_exec: String,
}

/// Resolve an icon file for an application-like path.
pub fn app_icon_path(file: &Path) -> Option<PathBuf> {
    if file.extension().is_some_and(|e| e.eq_ignore_ascii_case("appimage"))
        && let Some(icon) = adjacent_appimage_icon(file)
    {
        return Some(icon);
    }
    if let Some(entry) = desktop_entry_for(file)
        && let Some(path) = resolve_icon(&entry.icon)
    {
        return Some(path);
    }

    let stem = file.file_stem()?.to_string_lossy().to_lowercase();
    for ext in ["png", "svg"] {
        let candidate = file.with_extension(ext);
        if candidate.is_file() {
            return Some(candidate);
        }
        let sibling = file.parent()?.join(format!("{stem}.{ext}"));
        if sibling.is_file() {
            return Some(sibling);
        }
    }

    resolve_icon(&stem)
}

fn adjacent_appimage_icon(file: &Path) -> Option<PathBuf> {
    let root = file.parent()?.join("squashfs-root");
    if !root.is_dir() {
        return None;
    }
    let dir_icon = root.join(".DirIcon");
    if dir_icon.is_file() {
        return Some(dir_icon);
    }
    let mut best: Option<PathBuf> = None;
    let mut stack = vec![(root, 0u8)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 2 {
            continue;
        }
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for item in read.flatten() {
            let path = item.path();
            if path.is_dir() {
                stack.push((path, depth + 1));
            } else if path.extension().is_some_and(|e| e == "png" || e == "svg") {
                let score =
                    if path.file_name().is_some_and(|n| n.to_string_lossy().contains("icon")) {
                        2
                    } else {
                        1
                    };
                if best.is_none() || score > 1 {
                    best = Some(path);
                }
            }
        }
    }
    best
}

fn desktop_entry_for(file: &Path) -> Option<DesktopEntry> {
    if file.extension().is_some_and(|e| e.eq_ignore_ascii_case("desktop")) {
        return parse_desktop_file(file);
    }

    let file_name = file.file_name()?.to_string_lossy().to_lowercase();
    let stem = normalize(file.file_stem()?.to_string_lossy().as_ref());
    let path = file.to_string_lossy();
    let mut best: Option<(i32, DesktopEntry)> = None;

    for entry in desktop_cache() {
        let exec = format!("{} {}", entry.exec, entry.try_exec).to_lowercase();
        let mut score = 0;
        if !path.is_empty() && exec.contains(&path.to_lowercase()) {
            score = 100;
        } else if !file_name.is_empty() && exec.contains(&file_name) {
            score = 90;
        } else if !stem.is_empty() && exec.contains(&stem) {
            score = 60;
        }
        if entry.id == stem {
            score = score.max(80);
        }
        if normalize(&entry.name) == stem {
            score = score.max(50);
        }
        if score > 0 && best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, entry.clone()));
        }
    }
    best.map(|(_, entry)| entry)
}

fn normalize(value: &str) -> String {
    value.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn desktop_cache() -> &'static [DesktopEntry] {
    static CACHE: OnceLock<Vec<DesktopEntry>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut entries = Vec::new();
        for dir in application_dirs() {
            let Ok(read) = std::fs::read_dir(&dir) else { continue };
            for item in read.flatten() {
                let path = item.path();
                if path.extension().is_some_and(|e| e == "desktop")
                    && let Some(entry) = parse_desktop_file(&path)
                {
                    entries.push(entry);
                }
            }
        }
        entries
    })
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for base in data_dirs() {
        dirs.push(base.join("applications"));
        dirs.push(base.join("flatpak/exports/share/applications"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs.sort();
    dirs.dedup();
    dirs
}

fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::data_dir() {
        dirs.push(home);
    }
    let xdg = std::env::var_os("XDG_DATA_DIRS")
        .map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![PathBuf::from("/usr/local/share"), PathBuf::from("/usr/share")]);
    dirs.extend(xdg);
    dirs.sort();
    dirs.dedup();
    dirs
}

fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut entry = DesktopEntry {
        id: normalize(path.file_stem()?.to_string_lossy().as_ref()),
        ..DesktopEntry::default()
    };
    let mut in_main = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_main = line == "[Desktop Entry]";
            continue;
        }
        if !in_main {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        match key {
            "Name" => entry.name = value.trim().to_string(),
            "Icon" => entry.icon = value.trim().to_string(),
            "Exec" => entry.exec = value.trim().to_string(),
            "TryExec" => entry.try_exec = value.trim().to_string(),
            _ => {}
        }
    }
    if entry.icon.is_empty() { None } else { Some(entry) }
}

fn resolve_icon(icon: &str) -> Option<PathBuf> {
    let icon = icon.trim().trim_matches('"').trim_matches('\'');
    if icon.is_empty() {
        return None;
    }
    let direct = PathBuf::from(icon);
    if direct.is_file() {
        return Some(direct);
    }

    let stem = Path::new(icon)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| icon.to_string());
    let key = stem.to_lowercase();

    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(hit) = guard.get(&key)
    {
        return hit.clone();
    }

    let found = find_icon(&stem);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, found.clone());
    }
    found
}

fn find_icon(stem: &str) -> Option<PathBuf> {
    let themes = icon_themes();
    for base in icon_bases() {
        if base.file_name().is_some_and(|n| n == "pixmaps") {
            for ext in ["svg", "png"] {
                let candidate = base.join(format!("{stem}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            continue;
        }
        for theme in &themes {
            let theme_dir = base.join(theme);
            if !theme_dir.is_dir() {
                continue;
            }
            for rel in icon_rel_candidates() {
                let dir = if rel.is_empty() { theme_dir.clone() } else { theme_dir.join(rel) };
                for ext in ["svg", "png"] {
                    let candidate = dir.join(format!("{stem}.{ext}"));
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

fn icon_rel_candidates() -> Vec<&'static str> {
    vec![
        "scalable/apps",
        "512x512/apps",
        "256x256/apps",
        "128x128/apps",
        "96x96/apps",
        "64x64/apps",
        "48x48/apps",
        "32x32/apps",
        "24x24/apps",
        "16x16/apps",
        "apps/scalable",
        "apps/512",
        "apps/256",
        "apps/128",
        "apps/96",
        "apps/64",
        "apps/48",
        "apps/32",
        "apps/24",
        "apps/16",
        "apps",
        "applications",
        "",
    ]
}

fn icon_themes() -> Vec<String> {
    let mut themes = Vec::new();
    if let Some(theme) = current_theme() {
        themes.push(theme);
    }
    for theme in ["Adwaita", "hicolor", "breeze", "Papirus", "Yaru"] {
        if !themes.iter().any(|t| t == theme) {
            themes.push(theme.to_string());
        }
    }
    themes
}

fn icon_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    for data in data_dirs() {
        bases.push(data.join("icons"));
        bases.push(data.join("pixmaps"));
        bases.push(data.join("flatpak/exports/share/icons"));
    }
    if let Some(home) = dirs::home_dir() {
        bases.push(home.join(".icons"));
        bases.push(home.join(".local/share/icons"));
    }
    bases.push(PathBuf::from("/var/lib/flatpak/exports/share/icons"));
    bases.sort();
    bases.dedup();
    bases
}

fn current_theme() -> Option<String> {
    static THEME: OnceLock<Option<String>> = OnceLock::new();
    THEME
        .get_or_init(|| {
            let out = Command::new("gsettings")
                .args(["get", "org.gnome.desktop.interface", "icon-theme"])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let value = String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string();
            (!value.is_empty()).then_some(value)
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn desktop_entry_parses_main_section_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("demo.desktop");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "[Desktop Entry]\nName=Demo App\nIcon=/{}\nExec=/usr/bin/demo %U\n[Other]\nIcon=wrong",
            dir.path().join("demo.png").display()
        )
        .unwrap();
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.name, "Demo App");
        assert!(entry.icon.ends_with("demo.png"));
        assert_eq!(entry.exec, "/usr/bin/demo %U");
    }

    #[test]
    fn adjacent_icon_is_used_when_no_desktop_entry_matches() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Some.AppImage");
        std::fs::write(&app, b"x").unwrap();
        let icon = dir.path().join("some.svg");
        std::fs::write(&icon, b"<svg/>").unwrap();
        assert_eq!(app_icon_path(&app), Some(icon));
    }
}
