use crate::chip::{Chip, ChipKind};
use crate::date::DateSpec;
use crate::size::SizeSpec;
use shelf_core::{FileKind, Query, Sort};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filter {
    pub kinds: Vec<FileKind>,
    pub exts: Vec<String>,
    pub screenshots: bool,
    pub date: Option<DateSpec>,
    pub size: Option<SizeSpec>,
    pub sources: Vec<String>,
    pub tags: Vec<String>,
    /// Canonical folder name (e.g. `Downloads`); resolved against home into a path prefix.
    pub under: Option<String>,
    /// Words the parser did not understand; searched as filename substrings.
    pub text: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub filter: Filter,
    pub chips: Vec<Chip>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self == &Filter::default()
    }

    pub fn into_query(
        &self,
        home: &Path,
        sort: Sort,
        limit: u32,
        cursor: Option<String>,
        now: i64,
    ) -> Query {
        let (mtime_from, mtime_to) = self.date.map_or((None, None), |d| d.range(now));
        let (min_size, max_size) = self.size.map_or((None, None), |s| s.bounds());
        Query {
            kinds: self.kinds.clone(),
            exts: self.exts.clone(),
            mtime_from,
            mtime_to,
            min_size: min_size.map(|v| v as i64),
            max_size: max_size.map(|v| v as i64),
            sources: self.sources.clone(),
            names: Vec::new(),
            tags: self.tags.clone(),
            exclude_code: false,
            screenshot: self.screenshots.then_some(true),
            text: self.text.clone(),
            under_path: self.under.as_ref().map(|u| home.join(u).to_string_lossy().into_owned()),
            direct_children: false,
            in_trash: false,
            sort,
            limit,
            cursor,
        }
    }
}

/// What the parser decided to do with a phrase at one position.
enum Action {
    Kind(FileKind),
    Pdf,
    Screenshot,
    Date(DateSpec, String),
    Size(SizeSpec, String),
    Source(String),
    Place(String),
    Tag(String),
    Ext(String),
}

impl Action {
    fn chip(&self, now: i64) -> Chip {
        match self {
            Action::Kind(k) => {
                let (label, desc) = match k {
                    FileKind::Image => ("image", "Matches images"),
                    FileKind::Video => ("video", "Matches videos"),
                    FileKind::Music => ("music", "Matches audio files"),
                    FileKind::Document => ("document", "Matches documents"),
                    FileKind::Archive => ("archive", "Matches archives"),
                    FileKind::App => ("app", "Matches apps and installers"),
                    FileKind::Other => ("other", "Matches other files"),
                };
                Chip::new(ChipKind::Kind, label, desc)
            }
            Action::Pdf => Chip::new(ChipKind::Kind, "pdf", "Matches PDF files"),
            Action::Screenshot => Chip::new(
                ChipKind::Kind,
                "screenshot",
                "Matches screenshots by name, size and folder",
            ),
            Action::Date(spec, label) => {
                Chip::new(ChipKind::Date, label.clone(), spec.description(now))
            }
            Action::Size(spec, label) => {
                Chip::new(ChipKind::Size, label.clone(), spec.description())
            }
            Action::Source(s) => Chip::new(
                ChipKind::Source,
                format!("from {s}"),
                format!("Matches files that came from {s}"),
            ),
            Action::Place(p) => Chip::new(
                ChipKind::Place,
                format!("in {}", p.to_lowercase()),
                format!("Matches files inside your {p} folder"),
            ),
            Action::Tag(t) => {
                Chip::new(ChipKind::Tag, format!("tag:{t}"), format!("Matches files tagged {t}"))
            }
            Action::Ext(e) => {
                Chip::new(ChipKind::Kind, format!(".{e}"), format!("Matches .{e} files"))
            }
        }
    }

    fn apply(self, f: &mut Filter) {
        match self {
            Action::Kind(k) => {
                if !f.kinds.contains(&k) {
                    f.kinds.push(k);
                }
            }
            Action::Pdf => {
                if !f.exts.iter().any(|e| e == "pdf") {
                    f.exts.push("pdf".into());
                }
            }
            Action::Screenshot => f.screenshots = true,
            Action::Date(d, _) => f.date = Some(d),
            Action::Size(s, _) => f.size = Some(s),
            Action::Source(s) => f.sources.push(s),
            Action::Place(p) => f.under = Some(p),
            Action::Tag(t) => f.tags.push(t),
            Action::Ext(e) => {
                if !f.exts.contains(&e) {
                    f.exts.push(e);
                }
            }
        }
    }
}

pub fn parse(input: &str) -> Parsed {
    let lower = input.to_lowercase().replace(',', " ");
    let tokens: Vec<String> = lower
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| ",.;:!?\"'()[]".contains(c)).to_string())
        .filter(|w| !w.is_empty())
        .collect();

    let mut f = Filter::default();
    let mut chips = vec![];
    let now = chrono::Utc::now().timestamp();
    let mut i = 0;

    while i < tokens.len() {
        if let Some((n, action)) = match_at(&tokens, i) {
            // A second phrase in the same family (date / size / place) replaces
            // the first one; drop the superseded chip so what is shown matches
            // what is applied.
            if filter_had(&f, &action)
                && let Some(k) = family_of(&action)
            {
                chips.retain(|c: &Chip| c.kind != k);
            }
            chips.push(action.chip(now));
            action.apply(&mut f);
            i += n;
        } else if is_noise(&tokens[i]) {
            // Filler words carry no filter and no text; dropping them is the point.
            i += 1;
        } else {
            f.text.push(tokens[i].clone());
            i += 1;
        }
    }
    f.kinds.dedup();
    chips.dedup_by(|a, b| a.kind == b.kind && a.label == b.label);
    Parsed { filter: f, chips }
}

/// Filler words that carry no filter and should not be searched as text.
fn is_noise(w: &str) -> bool {
    // A single letter is a filename search, not filler: typing "a" must work.
    if w.chars().count() <= 1 {
        return false;
    }
    matches!(
        w,
        "the"
            | "a"
            | "an"
            | "of"
            | "my"
            | "files"
            | "file"
            | "show"
            | "find"
            | "me"
            | "please"
            | "all"
            | "modified"
            | "with"
            | "was"
            | "that"
            | "this"
            | "from"
            | "last"
            | "over"
            | "under"
            | "in"
    )
}

fn match_at(tokens: &[String], i: usize) -> Option<(usize, Action)> {
    let w = tokens[i].as_str();

    if let Some(tag) = w.strip_prefix("tag:").or_else(|| w.strip_prefix('#'))
        && !tag.is_empty()
    {
        return Some((1, Action::Tag(tag.to_string())));
    }

    // ---- single-word kinds and types
    if let Some(kind) = kind_word(w) {
        return Some((1, Action::Kind(kind)));
    }
    match w {
        "pdf" | "pdfs" => return Some((1, Action::Pdf)),
        _ if ext_word(w).is_some() => {
            return Some((1, Action::Ext(ext_word(w).unwrap_or_default().to_string())));
        }
        "screenshot" | "screenshots" => return Some((1, Action::Screenshot)),
        "today" => {
            return Some((1, Action::Date(DateSpec::Today, "today".into())));
        }
        "yesterday" => {
            return Some((1, Action::Date(DateSpec::Yesterday, "yesterday".into())));
        }
        _ => {}
    }

    // ---- two-word phrases
    if i + 1 < tokens.len() {
        let second = tokens[i + 1].as_str();
        match (w, second) {
            ("last", "week") => {
                return Some((2, Action::Date(DateSpec::LastWeek, "last week".into())));
            }
            ("this", "week") => {
                return Some((2, Action::Date(DateSpec::ThisWeek, "this week".into())));
            }
            ("last", "month") => {
                return Some((2, Action::Date(DateSpec::LastMonth, "last month".into())));
            }
            ("this", "month") => {
                return Some((2, Action::Date(DateSpec::ThisMonth, "this month".into())));
            }
            ("this", "year") => {
                return Some((2, Action::Date(DateSpec::ThisYear, "this year".into())));
            }
            ("last", "year") => {
                return Some((2, Action::Date(DateSpec::LastYear, "last year".into())));
            }
            ("in", m) => {
                if let Some(m) = month_index(m) {
                    let name = month_name(m);
                    // "in august 2024" — explicit year, 3 tokens.
                    if let Some(y) = tokens
                        .get(i + 2)
                        .and_then(|t| t.parse::<i32>().ok())
                        .filter(|y| (1900..2100).contains(y))
                    {
                        return Some((
                            3,
                            Action::Date(DateSpec::MonthYear(m, y), format!("in {name} {y}")),
                        ));
                    }
                    return Some((2, Action::Date(DateSpec::Month(m), format!("in {name}"))));
                }
                if let Some(place) = place_word(m) {
                    return Some((2, Action::Place(place.into())));
                }
            }
            _ => {}
        }
    }

    // ---- last N <unit>
    if w == "last"
        && i + 2 < tokens.len()
        && let Some(num) = parse_num(&tokens[i + 1])
    {
        let unit = tokens[i + 2].as_str();
        // Cap the window so absurd inputs cannot turn into near-everything
        // ranges; the label below reports the clamped value.
        let n = (num as u32).clamp(1, 3650);
        let (spec, plural): (DateSpec, bool) = match unit {
            "day" | "days" => (DateSpec::LastDays(n), n != 1),
            "week" | "weeks" => (DateSpec::LastWeeks(n), n != 1),
            "month" | "months" => (DateSpec::LastMonths(n), n != 1),
            _ => return None,
        };
        let label = if plural {
            format!("last {n} {}", tokens[i + 2].as_str())
        } else {
            let singular = match tokens[i + 2].as_str() {
                "days" => "day",
                "weeks" => "week",
                "months" => "month",
                u => u,
            };
            format!("last {n} {singular}")
        };
        return Some((3, Action::Date(spec, label)));
    }

    // ---- sizes: between X unit and Y unit
    if w == "between"
        && let Some((low, l1, low_num, low_unit)) = size_tail(tokens, i + 1)
        && tokens.get(i + 1 + l1).is_some_and(|t| t == "and")
        && let Some((high, l2, high_num, high_unit)) = size_tail(tokens, i + 2 + l1)
    {
        let (a, b) = if low <= high { (low, high) } else { (high, low) };
        let label = format!(
            "between {} and {}",
            fmt_size_label(low_num, low_unit),
            fmt_size_label(high_num, high_unit)
        );
        return Some((1 + l1 + 1 + l2, Action::Size(SizeSpec::Between(a, b), label)));
    }

    // ---- sizes: over|under N unit, larger|smaller than N unit
    if (w == "over" || w == "under")
        && let Some((bytes, len, num, unit)) = size_tail(tokens, i + 1)
    {
        let spec = if w == "over" { SizeSpec::Over(bytes) } else { SizeSpec::Under(bytes) };
        let label = format!("{} {}", w, fmt_size_label(num, unit));
        return Some((1 + len, Action::Size(spec, label)));
    }
    if (w == "larger" || w == "smaller")
        && i + 1 < tokens.len()
        && tokens[i + 1] == "than"
        && let Some((bytes, len, num, unit)) = size_tail(tokens, i + 2)
    {
        let spec = if w == "larger" { SizeSpec::Over(bytes) } else { SizeSpec::Under(bytes) };
        let label = format!("{w} than {}", fmt_size_label(num, unit));
        return Some((2 + len, Action::Size(spec, label)));
    }

    // ---- sources: from <site>
    if w == "from" && i + 1 < tokens.len() {
        let src = &tokens[i + 1];
        // Never swallow a token that starts another phrase ("from last week"),
        // or a month name that belongs to a date phrase ("from may").
        let phrase_starter = matches!(
            src.as_str(),
            "last"
                | "this"
                | "over"
                | "under"
                | "larger"
                | "smaller"
                | "in"
                | "today"
                | "yesterday"
                | "from"
        ) || parse_num(src).is_some()
            || month_index(src).is_some();
        if !phrase_starter {
            return Some((2, Action::Source(src.clone())));
        }
    }

    None
}

/// Size phrase body starting at `start`: (bytes, tokens used, typed number, typed unit).
fn size_tail(tokens: &[String], start: usize) -> Option<(u64, usize, f64, &'static str)> {
    tokens.get(start)?;
    // Attached unit ("5mb") takes one token; otherwise number + unit token.
    if let Some((num, unit)) = split_num_unit(&tokens[start]) {
        return Some((num_unit_bytes(num, unit), 1, num, unit));
    }
    let num = parse_num(&tokens[start])?;
    let unit_token = tokens.get(start + 1)?;
    let unit = unit_word(unit_token)?;
    Some((num_unit_bytes(num, unit), 2, num, unit))
}

fn fmt_size_label(num: f64, unit: &str) -> String {
    let n = if num.fract() == 0.0 { format!("{}", num as u64) } else { format!("{num}") };
    format!("{n} {unit}")
}

/// `"5mb"` → (5.0, "MB"); full-number tokens return None.
fn split_num_unit(token: &str) -> Option<(f64, &'static str)> {
    let digits_end = token.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(token.len());
    if digits_end == 0 || digits_end == token.len() {
        return None;
    }
    let num = token[..digits_end].parse::<f64>().ok()?;
    let unit = unit_word(&token[digits_end..])?;
    Some((num, unit))
}

fn parse_num(token: &str) -> Option<f64> {
    token.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0)
}

fn unit_word(token: &str) -> Option<&'static str> {
    match token {
        "b" | "byte" | "bytes" => Some("B"),
        "kb" | "kib" | "k" => Some("KB"),
        "mb" | "mib" | "m" => Some("MB"),
        "gb" | "gib" | "g" => Some("GB"),
        "tb" | "tib" | "t" => Some("TB"),
        _ => None,
    }
}

fn num_unit_bytes(num: f64, unit: &str) -> u64 {
    let mult = match unit {
        "B" => 1.0,
        "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => 1.0,
    };
    (num * mult) as u64
}

/// Which chip family an action belongs to, for superseding duplicate chips.
fn family_of(action: &Action) -> Option<ChipKind> {
    match action {
        Action::Date(..) => Some(ChipKind::Date),
        Action::Size(..) => Some(ChipKind::Size),
        Action::Place(..) => Some(ChipKind::Place),
        _ => None,
    }
}

/// Whether the filter already holds a value that this action would replace.
fn filter_had(f: &Filter, action: &Action) -> bool {
    match action {
        Action::Date(..) => f.date.is_some(),
        Action::Size(..) => f.size.is_some(),
        Action::Place(..) => f.under.is_some(),
        _ => false,
    }
}

fn ext_word(w: &str) -> Option<&'static str> {
    let ext = w.strip_prefix('.').unwrap_or(w);
    const EXTS: &[&str] = &[
        "7z", "aac", "ai", "apk", "appimage", "avif", "bash", "bmp", "bz2", "c", "cc", "cpp", "cs",
        "css", "csv", "deb", "dmg", "doc", "docx", "dng", "dts", "epub", "exe", "fish", "flac",
        "gif", "go", "gz", "h", "hpp", "heic", "html", "ico", "iso", "java", "jpeg", "jpg", "js",
        "json", "jsx", "key", "log", "m4a", "md", "mjs", "mkv", "mobi", "mov", "mp3", "mp4", "msi",
        "odp", "ods", "odt", "ogg", "otf", "pdf", "png", "ppt", "pptx", "py", "rar", "raw", "rpm",
        "rs", "sh", "snap", "sqlite", "svg", "tar", "tex", "tgz", "toml", "ts", "tsx", "ttf",
        "txt", "wav", "webm", "webp", "woff", "woff2", "xls", "xlsx", "xml", "yaml", "yml", "zip",
        "zsh", "zst",
    ];
    EXTS.iter().copied().find(|known| *known == ext)
}

fn kind_word(w: &str) -> Option<FileKind> {
    match w {
        "image" | "images" | "photo" | "photos" | "picture" | "pictures" | "pics" => {
            Some(FileKind::Image)
        }
        "video" | "videos" | "movie" | "movies" | "clip" | "clips" => Some(FileKind::Video),
        "music" | "song" | "songs" | "audio" | "track" | "tracks" => Some(FileKind::Music),
        "document" | "documents" | "doc" | "docs" | "spreadsheet" | "spreadsheets" => {
            Some(FileKind::Document)
        }
        "archive" | "archives" | "zip" | "zips" | "compressed" => Some(FileKind::Archive),
        "app" | "apps" | "application" | "applications" | "installer" | "installers" => {
            Some(FileKind::App)
        }
        _ => None,
    }
}

fn place_word(w: &str) -> Option<&'static str> {
    match w {
        "downloads" => Some("Downloads"),
        "documents" => Some("Documents"),
        "desktop" => Some("Desktop"),
        "pictures" => Some("Pictures"),
        "videos" => Some("Videos"),
        "music" => Some("Music"),
        "movies" => Some("Videos"),
        _ => None,
    }
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn month_index(w: &str) -> Option<u32> {
    if let Some(i) = MONTHS.iter().position(|m| *m == w) {
        return Some(i as u32 + 1);
    }
    if w.len() >= 3 {
        let matches: Vec<u32> = MONTHS
            .iter()
            .enumerate()
            .filter(|(_, m)| m.starts_with(w))
            .map(|(i, _)| i as u32 + 1)
            .collect();
        if matches.len() == 1 {
            return matches.into_iter().next();
        }
    }
    None
}

fn month_name(m: u32) -> &'static str {
    MONTHS[(m - 1) as usize]
}
