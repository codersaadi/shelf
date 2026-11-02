use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use shelf_core::{Sort, Store};
use shelf_query::parse;
use shelf_watcher::{Excludes, scan};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "shelf", about = "Local-first file intelligence", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Override the index location (default: $SHELF_DB or ~/.local/share/shelf/index.db).
    #[arg(long, global = true)]
    db: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Index a directory into the store.
    Index {
        /// Directory to index. Default: home.
        root: Option<PathBuf>,
        /// Rescan this root; paths that vanished are dropped.
        #[arg(long)]
        full: bool,
    },
    /// Run a plain-English query and print results.
    Query {
        /// e.g. "pdf in august over 5 MB from gmail"
        text: String,
        #[arg(long, default_value = "30")]
        limit: u32,
        /// Print JSON instead of the table.
        #[arg(long)]
        json: bool,
    },
    /// Show per-kind counts and totals.
    Stats,
    /// Show how a query parses (chips + filter), no database.
    Explain { text: String },
    /// Move files to the Trash (freedesktop spec).
    Trash { paths: Vec<PathBuf> },
    /// List reviewable cleanup findings.
    Cleanup {
        #[arg(long)]
        json: bool,
    },
    /// List Trash receipts.
    Receipts {
        #[arg(long)]
        json: bool,
    },
    /// Undo one Trash receipt.
    Undo { receipt: i64 },
    /// Show indexed metadata and tags for one path.
    Inspect {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Print the safe card snippet for a .txt or Markdown file.
    Text {
        path: PathBuf,
        /// Print the generated, sanitised HTML instead of plain text.
        #[arg(long)]
        html: bool,
    },
    /// Read or replace tags on one path (comma separated).
    Tags {
        path: PathBuf,
        #[arg(long)]
        set: Option<String>,
    },
    /// List saved searches.
    Saved {
        #[arg(long)]
        json: bool,
    },
    /// Create or replace a saved search.
    SaveSearch { name: String, query: String },
    /// Delete a saved search by id.
    DeleteSaved { id: i64 },
    /// Open a file with its default application, or --with a desktop app id.
    Open {
        path: PathBuf,
        #[arg(long)]
        with: Option<String>,
    },
    /// List the applications registered for a file.
    Apps { path: PathBuf },
    /// Resolve the native icon path for an application or desktop file.
    AppIcon { path: PathBuf },
    /// Copy files or directories into a destination folder.
    Copy { paths: Vec<PathBuf>, dest: PathBuf },
    /// Move files or directories into a destination folder.
    Move { paths: Vec<PathBuf>, dest: PathBuf },
    /// Open a terminal at a file's parent folder.
    Terminal { path: PathBuf },
    /// Set the default desktop application for a file's MIME type.
    Default { path: PathBuf, app: String },
    /// Show a file selected in the desktop file manager.
    Reveal { path: PathBuf },
    /// Write a cached thumbnail (image, video frame or PDF page) to a PNG.
    Thumb {
        path: PathBuf,
        #[arg(long, default_value = "256")]
        size: u32,
        #[arg(long, default_value = "thumbnail.png")]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = cli.db.unwrap_or_else(default_db);
    match cli.cmd {
        Cmd::Index { root, full } => {
            let root = root.unwrap_or_else(home);
            let store = Store::open(&db).context("opening index")?;
            let t = Instant::now();
            let n = scan(&store, &root, &Excludes::default(), |done| {
                eprintln!("indexed {done} files…");
            })?;
            if full {
                store.analyze()?;
            }
            let (count, bytes) = store.totals()?;
            println!(
                "scanned {n} files under {} in {:.2}s (index now: {count} files, {})",
                root.display(),
                t.elapsed().as_secs_f64(),
                fmt_bytes(bytes)
            );
        }
        Cmd::Query { text, limit, json } => {
            let store = Store::open(&db).context("opening index")?;
            let parsed = parse(&text);
            let now = chrono::Utc::now().timestamp();
            let q = parsed.filter.into_query(&home(), Sort::Modified, limit, None, now);
            let page = store.list(&q)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&page.items)?);
            } else {
                print_table(&page.items);
            }
        }
        Cmd::Stats => {
            let store = Store::open(&db).context("opening index")?;
            for s in store.kind_stats()? {
                println!("{:>10}  {:>9}  {}", s.count, fmt_bytes(s.bytes), s.kind.as_str());
            }
            let (count, bytes) = store.totals()?;
            println!("{:>10}  {:>9}  total", count, fmt_bytes(bytes));
        }
        Cmd::Explain { text } => {
            let parsed = parse(&text);
            for c in &parsed.chips {
                println!(
                    "chip  [{:^12}] {:<18} {}",
                    format!("{:?}", c.kind).to_lowercase(),
                    c.label,
                    c.description
                );
            }
            if !parsed.filter.text.is_empty() {
                println!("text  {:?} (searched in filenames)", parsed.filter.text);
            }
            if parsed.filter.text.is_empty() && parsed.chips.is_empty() {
                println!("(nothing recognized; the full text is searched in filenames)");
            }
        }
        Cmd::Cleanup { json } => {
            let store = Store::open(&db)?;
            let found = shelf_cleanup::findings(&store, &home())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&found)?);
            } else if found.is_empty() {
                println!("nothing to clean up");
            } else {
                for f in &found {
                    if let Some(command) = &f.command {
                        println!("{:<40} report only  {}  {command}", f.title, f.risk);
                    } else {
                        println!(
                            "{:<40} {:>9}  {:>3} files  {}",
                            f.title,
                            fmt_bytes(f.bytes),
                            f.count,
                            f.risk
                        );
                    }
                }
            }
        }
        Cmd::Receipts { json } => {
            let store = Store::open(&db)?;
            let receipts = store.list_receipts()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&receipts)?);
            } else {
                for r in receipts {
                    println!("#{:<4} {:<6} {}  undone={}", r.id, r.action, r.created, r.undone);
                }
            }
        }
        Cmd::Undo { receipt } => {
            let store = Store::open(&db)?;
            let entries: Vec<(PathBuf, PathBuf)> = store
                .receipt_paths(receipt)?
                .into_iter()
                .map(|(o, t)| (PathBuf::from(o), PathBuf::from(t)))
                .collect();
            let trash_paths: Vec<PathBuf> = entries.iter().map(|(_, t)| t.clone()).collect();
            let restored = shelf_platform::restore_receipt(&entries)?;
            store.mark_restored(receipt, &trash_paths)?;
            println!("restored {restored} files from receipt #{receipt}");
        }
        Cmd::Inspect { path, json } => {
            let store = Store::open(&db)?;
            let Some(file) = store.by_path(&path)? else {
                anyhow::bail!("{} is not in the index", path.display());
            };
            let tags = store.tags_of(&path)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "file": file, "tags": tags })
                    )?
                );
            } else {
                println!("{}", file.path);
                println!(
                    "  kind={} size={} modified={}",
                    file.kind.as_str(),
                    fmt_bytes(file.size),
                    file.mtime
                );
                println!("  tags={}", if tags.is_empty() { "—".into() } else { tags.join(", ") });
            }
        }
        Cmd::Text { path, html } => {
            let Some(snippet) = shelf_media::text_snippet(&path) else {
                anyhow::bail!("no text snippet for {}", path.display());
            };
            if html {
                print!("{}", snippet.html.as_deref().unwrap_or(&snippet.text));
            } else {
                println!("{}", snippet.text);
            }
        }
        Cmd::Tags { path, set } => {
            let store = Store::open(&db)?;
            if let Some(raw) = set {
                let tags: Vec<String> = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect();
                if path.exists() {
                    shelf_platform::write_tags_xattr(&path, &tags)?;
                }
                store.set_tags(&path, &tags)?;
            }
            let tags = store.tags_of(&path)?;
            println!("{}", if tags.is_empty() { "no tags".into() } else { tags.join(", ") });
        }
        Cmd::Saved { json } => {
            let store = Store::open(&db)?;
            let saved = store.list_saved_searches()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&saved)?);
            } else {
                for s in saved {
                    println!("#{:<3} {:<24} {}", s.id, s.name, s.query);
                }
            }
        }
        Cmd::SaveSearch { name, query } => {
            let store = Store::open(&db)?;
            let id = store.save_search(&name, &query)?;
            println!("saved search #{id} “{name}”");
        }
        Cmd::DeleteSaved { id } => {
            let store = Store::open(&db)?;
            store.delete_saved_search(id)?;
            println!("deleted saved search #{id}");
        }
        Cmd::Open { path, with } => match with {
            Some(app) => shelf_platform::open_with(&path, &app)
                .with_context(|| format!("opening {} with {app}", path.display()))?,
            None => shelf_platform::open_path(&path)
                .with_context(|| format!("opening {}", path.display()))?,
        },
        Cmd::Thumb { path, size, out } => {
            let bytes = shelf_media::thumbnail_png(&path, size)
                .with_context(|| format!("no thumbnail for {}", path.display()))?;
            std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
            println!("wrote {} ({} bytes)", out.display(), bytes.len());
        }
        Cmd::Copy { paths, dest } => {
            for path in shelf_platform::copy_paths(&paths, &dest)
                .with_context(|| format!("copying to {}", dest.display()))?
            {
                println!("{}", path.display());
            }
        }
        Cmd::Move { paths, dest } => {
            for path in shelf_platform::move_paths(&paths, &dest)
                .with_context(|| format!("moving to {}", dest.display()))?
            {
                println!("{}", path.display());
            }
        }
        Cmd::Terminal { path } => {
            let dir = path.parent().unwrap_or(std::path::Path::new("/"));
            shelf_platform::open_terminal(dir)
                .with_context(|| format!("opening terminal at {}", dir.display()))?;
        }
        Cmd::Default { path, app } => {
            shelf_platform::set_default_for(&path, &app)
                .with_context(|| format!("setting default for {}", path.display()))?;
            println!("default set for {}", path.display());
        }
        Cmd::Reveal { path } => {
            shelf_platform::show_in_file_manager(std::slice::from_ref(&path))
                .with_context(|| format!("revealing {}", path.display()))?;
        }
        Cmd::AppIcon { path } => match shelf_platform::app_icon_path(&path) {
            Some(icon) => println!("{}", icon.display()),
            None => anyhow::bail!("no native icon found for {}", path.display()),
        },
        Cmd::Apps { path } => {
            for (i, app) in shelf_platform::apps_for_path(&path).into_iter().enumerate() {
                println!(
                    "{:>2}. {:<28} {}{}",
                    i + 1,
                    app.name,
                    app.id,
                    if app.is_default { "  (default)" } else { "" }
                );
            }
        }
        Cmd::Trash { paths } => {
            let store = Store::open(&db).ok();
            let existing: Vec<PathBuf> = paths.into_iter().filter(|p| p.exists()).collect();
            if existing.is_empty() {
                anyhow::bail!("no such files");
            }
            let moved = shelf_platform::to_trash_many(&existing).context("moving to Trash")?;
            for m in &moved {
                println!("moved {} to Trash", m.original.display());
            }
            if let Some(store) = store {
                let pairs: Vec<(PathBuf, PathBuf)> =
                    moved.iter().map(|m| (m.original.clone(), m.trash_path.clone())).collect();
                store.mark_trashed(&pairs, Some("shelf trash"))?;
            }
        }
    }
    Ok(())
}

fn default_db() -> PathBuf {
    std::env::var_os("SHELF_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(home).join("shelf").join("index.db"))
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn print_table(items: &[shelf_core::FileMeta]) {
    if items.is_empty() {
        println!("no results");
        return;
    }
    println!("{:<44} {:>9}  {:<17} {:<10}", "NAME", "SIZE", "MODIFIED", "KIND");
    for f in items {
        let when = chrono::DateTime::from_timestamp(f.mtime, 0)
            .map(|utc| utc.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        println!(
            "{:<44} {:>9}  {:<17} {:<10}",
            truncate(&f.name, 42),
            fmt_bytes(f.size),
            when,
            f.kind.as_str()
        );
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let head = s.chars().take(n * 6 / 10).collect::<String>();
        let tail = s.chars().skip(s.chars().count() - n * 4 / 10).collect::<String>();
        format!("{head}…{tail}")
    }
}

fn fmt_bytes(n: u64) -> String {
    const KB: f64 = 1e3;
    const MB: f64 = 1e6;
    const GB: f64 = 1e9;
    let (div, unit) = if n as f64 >= GB {
        (GB, "GB")
    } else if n as f64 >= MB {
        (MB, "MB")
    } else if n as f64 >= KB {
        (KB, "KB")
    } else {
        (1.0, "B")
    };
    format!("{:.1} {unit}", n as f64 / div)
}
