use crate::error::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One application that can open a file, as advertised by the desktop's MIME
/// database. `id` is the desktop-file id (for example `vlc.desktop`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Applications registered for this file's MIME type, default first.
pub fn apps_for_path(path: &Path) -> Vec<AppInfo> {
    let Some(mime) = mime_of(path) else {
        return vec![];
    };
    apps_for_mime(&mime)
}

/// Applications registered for a MIME type, default first.
pub fn apps_for_mime(mime: &str) -> Vec<AppInfo> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    if let Ok(out) = Command::new("gio").args(["mime", mime]).output() {
        for id in parse_gio_mime(&String::from_utf8_lossy(&out.stdout)) {
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }
    if ids.is_empty()
        && let Some(id) = xdg_default(mime)
    {
        ids.push(id);
    }
    ids.into_iter()
        .enumerate()
        .map(|(i, id)| AppInfo { name: app_name(&id), id, is_default: i == 0 })
        .collect()
}

/// Make `app_id` the default handler for `path`'s MIME type.
pub fn set_default_for(path: &Path, app_id: &str) -> Result<()> {
    let Some(mime) = mime_of(path) else {
        return Err(crate::Error::NoApp(app_id.to_string()));
    };
    set_default_mime(&mime, app_id)
}

/// Make `app_id` the default handler for a MIME type.
pub fn set_default_mime(mime: &str, app_id: &str) -> Result<()> {
    if !is_launchable(app_id) || find_desktop(app_id).is_none() {
        return Err(crate::Error::NoApp(app_id.to_string()));
    }
    let status = Command::new("xdg-mime")
        .args(["default", app_id, mime])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if status.is_ok_and(|s| s.success()) {
        return Ok(());
    }
    let status = Command::new("gio")
        .args(["mime", mime, app_id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if status.is_ok_and(|s| s.success()) {
        Ok(())
    } else {
        Err(crate::Error::NoApp(app_id.to_string()))
    }
}

pub fn default_mime_app(mime: &str) -> Option<String> {
    xdg_default(mime)
}

pub fn mime_type(path: &Path) -> Option<String> {
    mime_of(path)
}

/// Launch `path` with the named desktop application.
pub fn open_with(path: &Path, app_id: &str) -> Result<()> {
    if !is_launchable(app_id) {
        return Err(crate::Error::NoApp(app_id.to_string()));
    }
    if let Some(desktop) = find_desktop(app_id)
        && launch("gio", &[std::ffi::OsStr::new("launch"), desktop.as_os_str(), path.as_os_str()])
    {
        return Ok(());
    }
    if launch("gtk-launch", &[app_id.as_ref(), path.as_os_str()]) {
        return Ok(());
    }
    // Some desktop entries are stale or live in an unreadable location. Keep
    // the user in control and open the default handler rather than doing nothing.
    match crate::dbus::open_path(path) {
        Ok(()) => Ok(()),
        Err(_) => Err(crate::Error::NoApp(app_id.to_string())),
    }
}

fn launch(cmd: &str, args: &[&std::ffi::OsStr]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Desktop files can also be passed directly (used by tests and power users).
pub fn app_name(id: &str) -> String {
    find_desktop(id)
        .as_deref()
        .and_then(desktop_name)
        .unwrap_or_else(|| id.trim_end_matches(".desktop").to_string())
}

fn mime_of(path: &Path) -> Option<String> {
    if let Ok(out) = Command::new("xdg-mime").args(["query", "filetype"]).arg(path).output()
        && out.status.success()
    {
        let mime = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !mime.is_empty() {
            return Some(mime);
        }
    }
    let out = Command::new("gio")
        .args(["info", "-a", "standard::content-type"])
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find_map(|line| line.trim().strip_prefix("standard::content-type:"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn xdg_default(mime: &str) -> Option<String> {
    let out = Command::new("xdg-mime").args(["query", "default", mime]).output().ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !id.is_empty()).then_some(id)
}

fn parse_gio_mime(text: &str) -> Vec<String> {
    let mut default = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Default application for ")
            && let Some((_, id)) = rest.rsplit_once(':')
        {
            default = Some(id.trim().to_string());
        }
    }

    let mut out = Vec::new();
    if let Some(id) = default
        && !id.is_empty()
    {
        out.push(id);
    }
    let mut registered = false;
    for line in text.lines() {
        let line = line.trim();
        if line == "Registered applications:" {
            registered = true;
        } else if line == "Recommended applications:" {
            registered = false;
        } else if registered && line.ends_with(".desktop") && !out.contains(&line.to_string()) {
            out.push(line.to_string());
        }
    }
    out
}

fn find_desktop(id: &str) -> Option<PathBuf> {
    if id.contains('/') {
        let p = PathBuf::from(id);
        return p.is_file().then_some(p);
    }
    desktop_dirs().into_iter().map(|dir| dir.join(id)).find(|p| p.is_file())
}

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data) = dirs::data_dir() {
        dirs.push(data.join("applications"));
    }
    if let Some(dirs_env) = std::env::var_os("XDG_DATA_DIRS") {
        for p in std::env::split_paths(&dirs_env) {
            dirs.push(p.join("applications"));
        }
    }
    for p in [
        "/usr/local/share/applications",
        "/usr/share/applications",
        "/var/lib/flatpak/exports/share/applications",
        "/var/lib/snapd/desktop/applications",
    ] {
        dirs.push(PathBuf::from(p));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    let mut seen = HashSet::new();
    dirs.retain(|p| seen.insert(p.clone()));
    dirs
}

fn desktop_name(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("Name="))
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
}

fn is_launchable(id: &str) -> bool {
    !id.is_empty() && !id.starts_with('-') && !id.contains('\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIO_SAMPLE: &str = "\
Default application for “video/mp4”: vlc.desktop
Registered applications:
\tvlc.desktop
\torg.gnome.Totem.desktop
Recommended applications:
\tvlc.desktop
";

    #[test]
    fn gio_mime_parser_reads_default_and_registered() {
        assert_eq!(parse_gio_mime(GIO_SAMPLE), vec!["vlc.desktop", "org.gnome.Totem.desktop"]);
    }

    #[test]
    fn gio_mime_parser_tolerates_missing_default() {
        assert_eq!(
            parse_gio_mime("Registered applications:\n\tmpv.desktop\n"),
            vec!["mpv.desktop"]
        );
    }

    #[test]
    fn app_name_reads_localized_desktop_name() {
        let dir = tempfile::tempdir().unwrap();
        let desktop = dir.path().join("demo.desktop");
        std::fs::write(&desktop, "[Desktop Entry]\nName=Demo Player\nName[de]=Demo\n").unwrap();
        assert_eq!(desktop_name(&desktop).as_deref(), Some("Demo Player"));
        assert_eq!(app_name(desktop.to_str().unwrap()), "Demo Player");
    }

    #[test]
    fn launchable_rejects_option_lookalikes() {
        assert!(is_launchable("vlc.desktop"));
        assert!(!is_launchable("-rf"));
        assert!(!is_launchable(""));
    }
}
