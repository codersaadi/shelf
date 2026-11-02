use serde::Serialize;
use shelf_core::{FileKind, FileMeta, Query, Sort, Store};
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

mod duplicates;

pub use duplicates::{DuplicateGroup, find_duplicates};

/// A reviewable set of files. Sizes and counts describe the files returned here,
/// not an estimate over unindexed or unreturned matches.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub reason: String,
    pub bytes: u64,
    pub count: usize,
    pub risk: String,
    /// Report-only system cleanup with a copyable command. No files to review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub truncated: bool,
    pub files: Vec<FileMeta>,
}

pub struct FindingSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub reason: &'static str,
    pub risk: &'static str,
    pub query: Query,
}

/// Findings from the index only: no root commands, cache walks or installed-package
/// guesses. Every card is a proposal the user reviews.
pub fn findings(store: &Store, home: &Path) -> shelf_core::Result<Vec<Finding>> {
    let now = chrono_now();
    let downloads = home.join("Downloads").to_string_lossy().into_owned();
    let mut found = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut inodes: HashSet<(u64, u64)> = HashSet::new();

    let installers = Query {
        exts: ["deb", "rpm", "appimage", "flatpak", "snap", "iso"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        under_path: Some(downloads.clone()),
        direct_children: true,
        min_size: Some(5 * 1024 * 1024),
        sort: Sort::Size,
        limit: 500,
        ..Query::default()
    };
    add(
        store,
        &mut found,
        &mut seen,
        &mut inodes,
        FindingSpec {
            id: "installers",
            title: "Installers in Downloads",
            reason: "Packages in Downloads. Review before moving anything.",
            risk: "low",
            query: installers,
        },
    )?;

    let old_screens = Query {
        kinds: vec![FileKind::Image],
        screenshot: Some(true),
        mtime_to: Some(now - 365 * 24 * 3600),
        sort: Sort::Size,
        limit: 500,
        ..Query::default()
    };
    add(
        store,
        &mut found,
        &mut seen,
        &mut inodes,
        FindingSpec {
            id: "old-screenshots",
            title: "Old screenshots",
            reason: "Screenshots from more than a year ago.",
            risk: "low",
            query: old_screens,
        },
    )?;

    let archives = Query {
        kinds: vec![FileKind::Archive],
        under_path: Some(downloads),
        direct_children: true,
        min_size: Some(50 * 1024 * 1024),
        sort: Sort::Size,
        limit: 500,
        ..Query::default()
    };
    add(
        store,
        &mut found,
        &mut seen,
        &mut inodes,
        FindingSpec {
            id: "download-archives",
            title: "Archives in Downloads",
            reason: "Compressed downloads. Keep one only if you still need what is inside.",
            risk: "low",
            query: archives,
        },
    )?;

    let large = Query {
        min_size: Some(1024 * 1024 * 1024),
        sort: Sort::Size,
        limit: 500,
        ..Query::default()
    };
    add(
        store,
        &mut found,
        &mut seen,
        &mut inodes,
        FindingSpec {
            id: "large-files",
            title: "Large files you may have forgotten",
            reason: "Over 1 GB each. Review before moving anything to the Trash.",
            risk: "medium",
            query: large,
        },
    )?;

    system_findings(&mut found);
    found.sort_by_key(|f| std::cmp::Reverse(f.bytes));
    Ok(found)
}

fn add(
    store: &Store,
    found: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
    inodes: &mut HashSet<(u64, u64)>,
    spec: FindingSpec,
) -> shelf_core::Result<()> {
    let page = store.list(&spec.query)?;
    let mut files = Vec::new();
    for f in page.items {
        if !seen.insert(f.path.clone()) {
            continue;
        }
        // Hardlinks are the same space on disk; count one path per inode.
        if let Ok(md) = std::fs::metadata(&f.path)
            && md.nlink() > 1
            && !inodes.insert((md.dev(), md.ino()))
        {
            continue;
        }
        files.push(f);
    }
    if files.is_empty() {
        return Ok(());
    }
    let bytes = files.iter().map(|f| f.size).sum();
    let count = files.len();
    found.push(Finding {
        id: spec.id.into(),
        title: spec.title.into(),
        reason: spec.reason.into(),
        bytes,
        count,
        risk: spec.risk.into(),
        command: None,
        truncated: page.next_cursor.is_some(),
        files,
    });
    Ok(())
}

fn system_findings(found: &mut Vec<Finding>) {
    let mut report = |id: &str, title: &str, reason: &str, command: &str, risk: &str| {
        let path = match id {
            "apt-cache" => "/var/cache/apt/archives",
            "dnf-cache" => "/var/cache/dnf",
            "pacman-cache" => "/var/cache/pacman/pkg",
            "journal" => "/var/log/journal",
            _ => return,
        };
        let bytes = dir_size(Path::new(path));
        if bytes == 0 {
            return;
        }
        found.push(Finding {
            id: id.into(),
            title: title.into(),
            reason: reason.into(),
            bytes,
            count: 0,
            risk: risk.into(),
            command: Some(command.into()),
            truncated: false,
            files: vec![],
        });
    };
    report(
        "apt-cache",
        "APT package cache",
        "System package cache. Shelf only reports this; run the command yourself.",
        "sudo apt clean",
        "low",
    );
    report(
        "dnf-cache",
        "DNF package cache",
        "System package cache. Shelf only reports this; run the command yourself.",
        "sudo dnf clean all",
        "low",
    );
    report(
        "pacman-cache",
        "Pacman package cache",
        "System package cache. Shelf only reports this; run the command yourself.",
        "sudo pacman -Sc",
        "low",
    );
    report(
        "journal",
        "System journal",
        "Journal logs. Shelf only reports this; run the command yourself.",
        "sudo journalctl --vacuum-time=30d",
        "medium",
    );
}

fn dir_size(path: &Path) -> u64 {
    if !path.is_dir() {
        return 0;
    }
    let Ok(out) = std::process::Command::new("du").args(["-sk"]).arg(path).output() else {
        return 0;
    };
    if !out.status.success() {
        return 0;
    }
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|kb| kb.saturating_mul(1024))
        .unwrap_or(0)
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
