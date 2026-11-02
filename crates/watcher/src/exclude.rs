use std::collections::HashSet;

/// Directory names and entry classes the index never enters (plan §11).
#[derive(Debug, Clone)]
pub struct Excludes {
    pub dir_names: HashSet<&'static str>,
    /// Skip dot-directories everywhere. Dotfiles are indexed because they are
    /// real user files (`.env`, `.gitignore`); desktop metadata is still skipped.
    pub skip_hidden: bool,
}

impl Default for Excludes {
    fn default() -> Self {
        Excludes {
            dir_names: HashSet::from([
                "node_modules",
                "target",
                "__pycache__",
                ".venv",
                "venv",
                "vendor",
                ".cache",
                ".git",
                ".hg",
                ".svn",
                ".tox",
                ".mypy_cache",
                ".pytest_cache",
                ".next",
                ".nuxt",
                "dist",
                "build",
                "DerivedData",
                "Pods",
                "gradle",
            ]),
            skip_hidden: true,
        }
    }
}

impl Excludes {
    pub fn skips_dir(&self, name: &str) -> bool {
        (self.skip_hidden && name.starts_with('.')) || self.dir_names.contains(name)
    }

    pub fn skips_entry(&self, name: &str) -> bool {
        if !self.skip_hidden {
            return false;
        }
        matches!(name, ".DS_Store" | "Thumbs.db" | "desktop.ini" | ".directory")
            || name.ends_with(".swp")
            || name.ends_with('~')
            || name.starts_with("~$")
    }

    /// Bundle roots are entered once, but only direct launchers are indexed.
    pub fn is_bundle_root(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        [".app", ".pkg", ".bundle", ".appdir", ".appimage"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
            || [
                "-x64",
                "-x86_64",
                "-amd64",
                "-arm64",
                "-aarch64",
                "-unpacked",
                "-linux64",
                "-linux-x64",
            ]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
    }
}
