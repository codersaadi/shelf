use crate::error::{Error, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Open a terminal emulator with its working directory set to `dir`.
pub fn open_terminal(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} is not a directory", dir.display()),
        )));
    }

    if let Ok(term) = std::env::var("TERMINAL") {
        let term = term.trim();
        if !term.is_empty()
            && Command::new(term)
                .current_dir(dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .is_ok()
        {
            return Ok(());
        }
    }

    let candidates: &[(&str, &[&str])] = &[
        ("ptyxis", &["--working-directory"]),
        ("gnome-terminal", &["--working-directory"]),
        ("kgx", &["--working-directory"]),
        ("konsole", &["--workdir"]),
        ("xfce4-terminal", &["--working-directory"]),
        ("tilix", &["--working-directory"]),
        ("kitty", &["--directory"]),
        ("alacritty", &["--working-directory"]),
        ("wezterm", &["start", "--cwd"]),
        ("foot", &["--working-directory"]),
    ];
    for (program, args) in candidates {
        let mut cmd = Command::new(program);
        cmd.current_dir(dir).args(*args).arg(dir);
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if cmd.spawn().is_ok() {
            return Ok(());
        }
    }
    Err(Error::NoTerminal)
}
