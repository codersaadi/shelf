use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Copy files or directories into `dest`, choosing non-colliding names.
pub fn copy_paths(paths: &[PathBuf], dest: &Path) -> Result<Vec<PathBuf>> {
    if !dest.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} is not a directory", dest.display()),
        )));
    }
    let mut out = Vec::with_capacity(paths.len());
    for source in paths {
        if !source.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} no longer exists", source.display()),
            )));
        }
        let target = unique_target(dest, source.file_name().unwrap_or_default());
        if source.is_dir() {
            copy_dir(source, &target)?;
        } else {
            std::fs::copy(source, &target)?;
        }
        out.push(target);
    }
    Ok(out)
}

/// Move files or directories into `dest`, falling back to copy+delete across
/// filesystems. Names are chosen so an existing file is never overwritten.
pub fn move_paths(paths: &[PathBuf], dest: &Path) -> Result<Vec<PathBuf>> {
    if !dest.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} is not a directory", dest.display()),
        )));
    }
    let mut out = Vec::with_capacity(paths.len());
    for source in paths {
        if !source.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} no longer exists", source.display()),
            )));
        }
        let target = unique_target(dest, source.file_name().unwrap_or_default());
        if std::fs::rename(source, &target).is_err() {
            if source.is_dir() {
                copy_dir(source, &target)?;
                std::fs::remove_dir_all(source)?;
            } else {
                std::fs::copy(source, &target)?;
                std::fs::remove_file(source)?;
            }
        }
        out.push(target);
    }
    Ok(out)
}

fn unique_target(dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for n in 2..10_000 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-copy{ext}"))
}

fn copy_dir(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else if from.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_never_overwrites_an_existing_name() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        let a = src.join("note.txt");
        std::fs::write(&a, "new").unwrap();
        std::fs::write(dest.join("note.txt"), "old").unwrap();

        let out = copy_paths(&[a], &dest).unwrap();
        assert_eq!(out[0].file_name().unwrap(), "note (2).txt");
        assert_eq!(std::fs::read_to_string(dest.join("note.txt")).unwrap(), "old");
    }

    #[test]
    fn move_refuses_a_missing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(move_paths(&[file], &dir.path().join("missing")).is_err());
    }
}
