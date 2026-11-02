use serde::Serialize;
use sha2::{Digest, Sha256};
use shelf_core::{FileMeta, Store};
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub size: u64,
    pub wasted: u64,
    pub files: Vec<FileMeta>,
}

pub fn find_duplicates(store: &Store) -> shelf_core::Result<Vec<DuplicateGroup>> {
    let candidates = store.duplicate_candidates()?;
    let mut by_size: HashMap<u64, Vec<FileMeta>> = HashMap::new();
    for file in candidates {
        by_size.entry(file.size).or_default().push(file);
    }

    let mut by_hash: HashMap<(u64, String), Vec<FileMeta>> = HashMap::new();
    for (size, files) in by_size {
        if files.len() < 2 {
            continue;
        }
        for file in files {
            if let Some(hash) = hash_file(&file.path) {
                by_hash.entry((size, hash)).or_default().push(file);
            }
        }
    }

    let mut groups = Vec::new();
    for ((size, _), mut files) in by_hash {
        if files.len() < 2 {
            continue;
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let mut inodes = std::collections::HashSet::new();
        for file in &files {
            if let Ok(meta) = std::fs::metadata(&file.path) {
                inodes.insert((meta.dev(), meta.ino()));
            }
        }
        let copies = inodes.len().max(1) as u64;
        groups.push(DuplicateGroup { size, wasted: size.saturating_mul(copies - 1), files });
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group.wasted));
    groups.truncate(100);
    Ok(groups)
}

fn hash_file(path: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(format!("{:x}", hasher.finalize()))
}
