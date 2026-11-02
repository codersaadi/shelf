#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Table-driven parser suite (target: 100 cases, per AGENTS.md).
//! Naming: `parse_<input>_<expected>` applied to each row via the case id.

use shelf_core::FileKind;
use shelf_query::{DateSpec, Filter, SizeSpec, parse};

#[derive(Default)]
struct Exp {
    text: Vec<&'static str>,
    kinds: Vec<FileKind>,
    exts: Vec<&'static str>,
    screenshots: bool,
    date: Option<DateSpec>,
    size: Option<SizeSpec>,
    sources: Vec<&'static str>,
    tags: Vec<&'static str>,
    under: Option<&'static str>,
    chips: Vec<&'static str>,
}

fn exp(text: &[&'static str]) -> Exp {
    Exp { text: text.to_vec(), ..Default::default() }
}

fn cases() -> Vec<(&'static str, Exp)> {
    let mut c: Vec<(&'static str, Exp)> = Vec::with_capacity(120);

    // ---- flagship queries (plan §7 examples)
    c.push(("pdf", exp(&[]).with_exts(&["pdf"]).with_chips(&["pdf"])));
    c.push((
        "tag:work pdf",
        exp(&[]).with_exts(&["pdf"]).with_tags(&["work"]).with_chips(&["tag:work", "pdf"]),
    ));
    c.push((" #urgent", exp(&[]).with_tags(&["urgent"]).with_chips(&["tag:urgent"])));
    c.push(("apk", exp(&[]).with_exts(&["apk"]).with_chips(&[".apk"])));
    c.push(("apk invoice", exp(&["invoice"]).with_exts(&["apk"]).with_chips(&[".apk"])));
    c.push(("this year", exp(&[]).with_date(Some(DateSpec::ThisYear)).with_chips(&["this year"])));
    c.push(("last year", exp(&[]).with_date(Some(DateSpec::LastYear)).with_chips(&["last year"])));
    c.push((
        "between 1 MB and 5 MB",
        exp(&[])
            .with_size(Some(SizeSpec::Between(1_000_000, 5_000_000)))
            .with_chips(&["between 1 MB and 5 MB"]),
    ));
    c.push((
        "pdf in august",
        exp(&[])
            .with_exts(&["pdf"])
            .with_date(Some(DateSpec::Month(8)))
            .with_chips(&["pdf", "in august"]),
    ));
    c.push((
        "pdf in august over 5 MB",
        exp(&[])
            .with_exts(&["pdf"])
            .with_date(Some(DateSpec::Month(8)))
            .with_size(Some(SizeSpec::Over(5_000_000)))
            .with_chips(&["pdf", "in august", "over 5 MB"]),
    ));
    c.push((
        "pdf in august over 5 MB from gmail",
        exp(&[])
            .with_exts(&["pdf"])
            .with_date(Some(DateSpec::Month(8)))
            .with_size(Some(SizeSpec::Over(5_000_000)))
            .with_sources(&["gmail"])
            .with_chips(&["pdf", "in august", "over 5 MB", "from gmail"]),
    ));
    c.push((
        "pdf in august over 5 MB from gmail invoice",
        exp(&["invoice"])
            .with_exts(&["pdf"])
            .with_date(Some(DateSpec::Month(8)))
            .with_size(Some(SizeSpec::Over(5_000_000)))
            .with_sources(&["gmail"])
            .with_chips(&["pdf", "in august", "over 5 MB", "from gmail"]),
    ));
    c.push((
        "screenshots from last week",
        exp(&[])
            .with_screenshots(true)
            .with_date(Some(DateSpec::LastWeek))
            .with_chips(&["screenshot", "last week"]),
    ));
    c.push((
        "show me all pdf files modified today",
        exp(&[]).with_exts(&["pdf"]).with_date(Some(DateSpec::Today)).with_chips(&["pdf", "today"]),
    ));

    // ---- kind words
    for (w, k) in [
        ("image", FileKind::Image),
        ("images", FileKind::Image),
        ("photos", FileKind::Image),
        ("picture", FileKind::Image),
        ("video", FileKind::Video),
        ("videos", FileKind::Video),
        ("movie", FileKind::Video),
        ("music", FileKind::Music),
        ("songs", FileKind::Music),
        ("audio", FileKind::Music),
        ("document", FileKind::Document),
        ("docs", FileKind::Document),
        ("archive", FileKind::Archive),
        ("zips", FileKind::Archive),
        ("app", FileKind::App),
        ("apps", FileKind::App),
        ("installer", FileKind::App),
        ("installers", FileKind::App),
    ] {
        let label = match k {
            FileKind::Image => "image",
            FileKind::Video => "video",
            FileKind::Music => "music",
            FileKind::Document => "document",
            FileKind::Archive => "archive",
            FileKind::App => "app",
            FileKind::Other => "other",
        };
        c.push((w, exp(&[]).with_kinds(&[k]).with_chips(&[label])));
    }

    // ---- screenshot
    c.push(("screenshot", exp(&[]).with_screenshots(true).with_chips(&["screenshot"])));
    c.push(("screenshots", exp(&[]).with_screenshots(true).with_chips(&["screenshot"])));

    // ---- dates: fixed phrases
    c.push(("today", exp(&[]).with_date(Some(DateSpec::Today)).with_chips(&["today"])));
    c.push(("yesterday", exp(&[]).with_date(Some(DateSpec::Yesterday)).with_chips(&["yesterday"])));
    c.push(("last week", exp(&[]).with_date(Some(DateSpec::LastWeek)).with_chips(&["last week"])));
    c.push(("this week", exp(&[]).with_date(Some(DateSpec::ThisWeek)).with_chips(&["this week"])));
    c.push((
        "last month",
        exp(&[]).with_date(Some(DateSpec::LastMonth)).with_chips(&["last month"]),
    ));
    c.push((
        "this month",
        exp(&[]).with_date(Some(DateSpec::ThisMonth)).with_chips(&["this month"]),
    ));

    // ---- dates: in <month>, full names
    for (m, name) in [
        (1u32, "january"),
        (2, "february"),
        (3, "march"),
        (4, "april"),
        (5, "may"),
        (6, "june"),
        (7, "july"),
        (8, "august"),
        (9, "september"),
        (10, "october"),
        (11, "november"),
        (12, "december"),
    ] {
        let input: &'static str = Box::leak(format!("in {name}").into_boxed_str());
        let label: &'static str = Box::leak(format!("in {name}").into_boxed_str());
        c.push((input, exp(&[]).with_date(Some(DateSpec::Month(m))).with_chips(&[label])));
    }

    // ---- dates: 3-letter prefixes (chips normalize to the full month name)
    for (m, short) in [(8u32, "aug"), (9, "sep"), (10, "oct"), (12, "dec")] {
        let input: &'static str = Box::leak(format!("in {short}").into_boxed_str());
        let label: &'static str = Box::leak(format!("in {}", month_full(m)).into_boxed_str());
        c.push((input, exp(&[]).with_date(Some(DateSpec::Month(m))).with_chips(&[label])));
    }

    // ---- dates: in <month> <year>
    c.push((
        "invoices in august 2024",
        exp(&[])
            .with_date(Some(DateSpec::MonthYear(8, 2024)))
            .with_text(&["invoices"])
            .with_chips(&["in august 2024"]),
    ));

    // ---- dates: last N units
    for (input, spec, label) in [
        ("last 7 days", DateSpec::LastDays(7), "last 7 days"),
        ("last 30 days", DateSpec::LastDays(30), "last 30 days"),
        ("last 2 weeks", DateSpec::LastWeeks(2), "last 2 weeks"),
        ("last 3 months", DateSpec::LastMonths(3), "last 3 months"),
        ("modified in the last 14 days", DateSpec::LastDays(14), "last 14 days"),
    ] {
        c.push((input, exp(&[]).with_date(Some(spec)).with_chips(&[label])));
    }

    // ---- sizes: over/under with detached units
    for (input, spec, label) in [
        ("over 5 mb", SizeSpec::Over(5_000_000), "over 5 MB"),
        ("over 1 gb", SizeSpec::Over(1_000_000_000), "over 1 GB"),
        ("over 0.5 gb", SizeSpec::Over(500_000_000), "over 0.5 GB"),
        ("under 1 kb", SizeSpec::Under(1_000), "under 1 KB"),
        ("under 500 b", SizeSpec::Under(500), "under 500 B"),
        ("over 2 tb", SizeSpec::Over(2_000_000_000_000), "over 2 TB"),
    ] {
        c.push((input, exp(&[]).with_size(Some(spec)).with_chips(&[label])));
    }

    // ---- sizes: attached units and comparative phrasing
    c.push((
        "videos over 5mb",
        exp(&[])
            .with_kinds(&[FileKind::Video])
            .with_size(Some(SizeSpec::Over(5_000_000)))
            .with_chips(&["video", "over 5 MB"]),
    ));
    c.push((
        "larger than 2 gb",
        exp(&[]).with_size(Some(SizeSpec::Over(2_000_000_000))).with_chips(&["larger than 2 GB"]),
    ));
    c.push((
        "smaller than 500 kb",
        exp(&[]).with_size(Some(SizeSpec::Under(500_000))).with_chips(&["smaller than 500 KB"]),
    ));

    // ---- sources
    for src in ["gmail", "github", "slack"] {
        let input: &'static str = Box::leak(format!("from {src}").into_boxed_str());
        let label: &'static str = Box::leak(format!("from {src}").into_boxed_str());
        c.push((input, exp(&[]).with_sources(&[src]).with_chips(&[label])));
    }

    // ---- places
    for (word, folder) in [
        ("downloads", "Downloads"),
        ("documents", "Documents"),
        ("desktop", "Desktop"),
        ("pictures", "Pictures"),
        ("videos", "Videos"),
        ("music", "Music"),
        ("movies", "Videos"),
    ] {
        let input: &'static str = Box::leak(format!("in {word}").into_boxed_str());
        let label: &'static str =
            Box::leak(format!("in {}", folder.to_lowercase()).into_boxed_str());
        c.push((input, exp(&[]).with_under(Some(folder)).with_chips(&[label])));
    }

    // ---- compounds and edge cases
    c.push((
        "images over 10 mb in downloads",
        exp(&[])
            .with_kinds(&[FileKind::Image])
            .with_size(Some(SizeSpec::Over(10_000_000)))
            .with_under(Some("Downloads"))
            .with_chips(&["image", "over 10 MB", "in downloads"]),
    ));
    c.push((
        "pdf invoice 0387",
        exp(&["0387"]).with_exts(&["pdf"]).with_text(&["invoice", "0387"]).with_chips(&["pdf"]),
    ));
    c.push(("meeting-notes", exp(&["meeting-notes"])));
    c.push(("PDF", exp(&[]).with_exts(&["pdf"]).with_chips(&["pdf"])));
    c.push(("PDFs", exp(&[]).with_exts(&["pdf"]).with_chips(&["pdf"])));
    c.push(("pdf,pdf", exp(&[]).with_exts(&["pdf"]).with_chips(&["pdf"])));
    c.push((
        "the report from last week",
        exp(&["report"]).with_date(Some(DateSpec::LastWeek)).with_chips(&["last week"]),
    ));
    c.push((
        "video from github",
        exp(&[])
            .with_kinds(&[FileKind::Video])
            .with_sources(&["github"])
            .with_chips(&["video", "from github"]),
    ));
    c.push((
        "music under 10 mb",
        exp(&[])
            .with_kinds(&[FileKind::Music])
            .with_size(Some(SizeSpec::Under(10_000_000)))
            .with_chips(&["music", "under 10 MB"]),
    ));
    c.push((
        "screenshots over 2 mb last 30 days",
        exp(&[])
            .with_screenshots(true)
            .with_size(Some(SizeSpec::Over(2_000_000)))
            .with_date(Some(DateSpec::LastDays(30)))
            .with_chips(&["screenshot", "over 2 MB", "last 30 days"]),
    ));
    c.push((
        "documents in august over 5 mb from gmail lease",
        exp(&[])
            .with_kinds(&[FileKind::Document])
            .with_date(Some(DateSpec::Month(8)))
            .with_size(Some(SizeSpec::Over(5_000_000)))
            .with_sources(&["gmail"])
            .with_text(&["lease"])
            .with_chips(&["document", "in august", "over 5 MB", "from gmail"]),
    ));
    c.push((
        "photos in june over 3 mb",
        exp(&[])
            .with_kinds(&[FileKind::Image])
            .with_date(Some(DateSpec::Month(6)))
            .with_size(Some(SizeSpec::Over(3_000_000)))
            .with_chips(&["image", "in june", "over 3 MB"]),
    ));
    c.push((
        "invoices pdf in december under 1 mb",
        exp(&["invoices"])
            .with_exts(&["pdf"])
            .with_date(Some(DateSpec::Month(12)))
            .with_size(Some(SizeSpec::Under(1_000_000)))
            .with_chips(&["pdf", "in december", "under 1 MB"]),
    ));

    // ---- additional kind words
    for (w, k) in [
        ("photo", FileKind::Image),
        ("pictures", FileKind::Image),
        ("clips", FileKind::Video),
        ("movies", FileKind::Video),
        ("audio", FileKind::Music),
        ("tracks", FileKind::Music),
        ("spreadsheets", FileKind::Document),
        ("compressed", FileKind::Archive),
        ("applications", FileKind::App),
    ] {
        let label = match k {
            FileKind::Image => "image",
            FileKind::Video => "video",
            FileKind::Music => "music",
            FileKind::Document => "document",
            FileKind::Archive => "archive",
            FileKind::App => "app",
            FileKind::Other => "other",
        };
        c.push((w, exp(&[]).with_kinds(&[k]).with_chips(&[label])));
    }

    // ---- additional compounds
    c.push((
        "last 1 day",
        exp(&[]).with_date(Some(DateSpec::LastDays(1))).with_chips(&["last 1 day"]),
    ));
    c.push((
        "under 100 mb in downloads",
        exp(&[])
            .with_size(Some(SizeSpec::Under(100_000_000)))
            .with_under(Some("Downloads"))
            .with_chips(&["under 100 MB", "in downloads"]),
    ));
    c.push((
        "over 999 b",
        exp(&[]).with_size(Some(SizeSpec::Over(999))).with_chips(&["over 999 B"]),
    ));
    c.push((
        "zip archives in downloads",
        exp(&[])
            .with_kinds(&[FileKind::Archive])
            .with_under(Some("Downloads"))
            .with_chips(&["archive", "in downloads"]),
    ));

    // ---- superseding phrases: the last chip wins and the stale one goes away
    c.push((
        "over 5 mb under 1 gb",
        exp(&[]).with_size(Some(SizeSpec::Under(1_000_000_000))).with_chips(&["under 1 GB"]),
    ));
    c.push((
        "in downloads in pictures",
        exp(&[]).with_under(Some("Pictures")).with_chips(&["in pictures"]),
    ));
    c.push((
        "today in august",
        exp(&[]).with_date(Some(DateSpec::Month(8))).with_chips(&["in august"]),
    ));

    // ---- extreme windows clamp to a sane range and report it
    c.push((
        "last 4294967297 days",
        exp(&[]).with_date(Some(DateSpec::LastDays(3650))).with_chips(&["last 3650 days"]),
    ));

    // ---- month names are dates, not sources, after "from"
    c.push((
        "pdfs from may",
        exp(&[]).with_exts(&["pdf"]).with_text(&["may"]).with_chips(&["pdf"]),
    ));

    c
}

fn month_full(m: u32) -> &'static str {
    const FULL: [&str; 12] = [
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
    FULL[(m - 1) as usize]
}

impl Exp {
    fn with_text(mut self, t: &[&'static str]) -> Exp {
        self.text = t.to_vec();
        self
    }
    fn with_kinds(mut self, k: &[FileKind]) -> Exp {
        self.kinds = k.to_vec();
        self
    }
    fn with_exts(mut self, e: &[&'static str]) -> Exp {
        self.exts = e.to_vec();
        self
    }
    fn with_screenshots(mut self, s: bool) -> Exp {
        self.screenshots = s;
        self
    }
    fn with_date(mut self, d: Option<DateSpec>) -> Exp {
        self.date = d;
        self
    }
    fn with_size(mut self, s: Option<SizeSpec>) -> Exp {
        self.size = s;
        self
    }
    fn with_sources(mut self, s: &[&'static str]) -> Exp {
        self.sources = s.to_vec();
        self
    }
    fn with_tags(mut self, t: &[&'static str]) -> Exp {
        self.tags = t.to_vec();
        self
    }
    fn with_under(mut self, u: Option<&'static str>) -> Exp {
        self.under = u;
        self
    }
    fn with_chips(mut self, labels: &[&'static str]) -> Exp {
        self.chips = labels.to_vec();
        self
    }
}

#[test]
fn single_letters_are_searchable_text() {
    let p = parse("a");
    assert_eq!(p.filter.text, vec!["a".to_string()]);
    assert!(p.filter.kinds.is_empty());
    let p = parse("x");
    assert_eq!(p.filter.text, vec!["x".to_string()]);
}

#[test]
fn suite_has_100_cases() {
    assert!(
        cases().len() >= 100,
        "parser suite target is at least 100 cases, got {}",
        cases().len()
    );
}

#[test]
fn parse_cases_match_expected_filters() {
    for (i, (input, want)) in cases().iter().enumerate() {
        let got = parse(input);
        let f = &got.filter;
        assert_eq!(f.text, want.text, "case {i} ({input:?}) text mismatch");
        assert_eq!(f.kinds, want.kinds, "case {i} ({input:?}) kinds mismatch");
        assert_eq!(
            f.exts.iter().map(String::as_str).collect::<Vec<_>>(),
            want.exts,
            "case {i} ({input:?}) exts mismatch"
        );
        assert_eq!(f.screenshots, want.screenshots, "case {i} ({input:?}) screenshots mismatch");
        assert_eq!(f.date, want.date, "case {i} ({input:?}) date mismatch");
        assert_eq!(f.size, want.size, "case {i} ({input:?}) size mismatch");
        assert_eq!(
            f.sources.iter().map(String::as_str).collect::<Vec<_>>(),
            want.sources,
            "case {i} ({input:?}) sources mismatch"
        );
        assert_eq!(
            f.tags.iter().map(String::as_str).collect::<Vec<_>>(),
            want.tags,
            "case {i} ({input:?}) tags mismatch"
        );
        assert_eq!(f.under.as_deref(), want.under, "case {i} ({input:?}) under mismatch");
        let labels: Vec<&str> = got.chips.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, want.chips, "case {i} ({input:?}) chips mismatch");
    }
}

#[test]
fn parse_nothing_dropped_on_gibberish() {
    let p = parse("xylophone qwerty 42");
    assert_eq!(p.filter.text, vec!["xylophone", "qwerty", "42"]);
    assert!(p.chips.is_empty());
}

#[test]
fn parse_lone_connectors_become_noise_not_text() {
    let p = parse("report from last");
    assert_eq!(p.filter.text, vec!["report"]);
}

#[test]
fn date_ranges_are_sane() {
    let now = 1_770_000_000; // fixed reference second
    let (from, to) = DateSpec::Today.range(now);
    assert!(from.unwrap() <= now && to.unwrap() >= now);
    let (from, to) = DateSpec::LastWeek.range(now);
    assert_eq!(to.unwrap() - from.unwrap(), 7 * 86_400 - 1);
}

#[test]
fn filter_maps_into_query() {
    let f = Filter {
        kinds: vec![FileKind::Document],
        exts: vec!["pdf".into()],
        date: Some(DateSpec::Month(8)),
        size: Some(SizeSpec::Over(5_000_000)),
        sources: vec!["gmail".into()],
        ..Default::default()
    };
    let now = 1_770_000_000;
    let q =
        f.into_query(std::path::Path::new("/home/u"), shelf_core::Sort::Modified, 50, None, now);
    assert_eq!(q.kinds, vec![FileKind::Document]);
    assert_eq!(q.exts, vec!["pdf"]);
    assert!(q.mtime_from.is_some());
    assert_eq!(q.min_size, Some(5_000_001));
    assert_eq!(q.sources, vec!["gmail"]);
}
