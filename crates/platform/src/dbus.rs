use crate::error::Result;
use std::path::Path;

/// Open the file manager with `paths` selected (GNOME Files, Dolphin, …).
/// Falls back to opening the parent folder when the D-Bus service is absent.
pub fn show_in_file_manager(paths: &[std::path::PathBuf]) -> Result<()> {
    let uris: Vec<String> = paths.iter().map(|p| path_uri(p.as_path())).collect();
    let conn = zbus::blocking::Connection::session()?;
    let reply = conn.call_method(
        Some("org.freedesktop.FileManager1"),
        "/org/freedesktop/FileManager1",
        Some("org.freedesktop.FileManager1"),
        "ShowItems",
        &(&uris, ""),
    );
    match reply {
        Ok(_) => Ok(()),
        Err(err) => {
            // No file manager service (common on sway/XFCE): open the parent instead.
            tracing::debug!("ShowItems unavailable ({err}); falling back to parent folder");
            if let Some(parent) = paths.first().map(|p| p.as_path()).and_then(Path::parent) {
                open_uri(&path_uri(parent))?;
            }
            Ok(())
        }
    }
}

/// Open a file with its default application. `gio open` is the most reliable
/// path on a normal Linux session; `xdg-open` and the OpenURI portal are
/// fallbacks. Portal calls are preferred under Flatpak, but they can return a
/// handle without ever showing a window on minimal compositors, so they are last.
pub fn open_path(path: &Path) -> Result<()> {
    // Inside Flatpak the portal is the only sanctioned opener; spawning `gio`
    // would run against the sandbox's view of the session.
    if std::env::var_os("FLATPAK_ID").is_some() {
        return open_uri(&path_uri(path));
    }
    let spawn = |cmd: &str, args: &[&std::ffi::OsStr]| {
        std::process::Command::new(cmd)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
    };
    let path_arg = path.as_os_str();
    if spawn("gio", &[std::ffi::OsStr::new("open"), path_arg]) {
        return Ok(());
    }
    if spawn("xdg-open", &[path_arg]) {
        return Ok(());
    }
    open_uri(&path_uri(path))
}

fn open_uri(uri: &str) -> Result<()> {
    let conn = zbus::blocking::Connection::session()?;
    conn.call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.portal.OpenURI"),
        "OpenURI",
        &("", uri, std::collections::HashMap::<String, zbus::zvariant::Value>::new()),
    )?;
    Ok(())
}

fn path_uri(path: &Path) -> String {
    let bytes = path.as_os_str().as_encoded_bytes();
    let mut out = String::from("file://");
    for &b in bytes {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'/'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b':'
            | b'@' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
