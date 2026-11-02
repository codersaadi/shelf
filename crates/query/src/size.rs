#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeSpec {
    Over(u64),
    Under(u64),
    Between(u64, u64),
}

impl SizeSpec {
    pub fn bounds(&self) -> (Option<u64>, Option<u64>) {
        match self {
            SizeSpec::Over(n) => (Some(n + 1), None),
            SizeSpec::Under(n) => (None, Some(n.saturating_sub(1))),
            SizeSpec::Between(a, b) => (Some(*a), Some(*b)),
        }
    }

    pub fn description(&self) -> String {
        match self {
            SizeSpec::Over(n) => format!("Matches files larger than {}", fmt_bytes(*n)),
            SizeSpec::Under(n) => format!("Matches files smaller than {}", fmt_bytes(*n)),
            SizeSpec::Between(a, b) => {
                format!("Matches files between {} and {}", fmt_bytes(*a), fmt_bytes(*b))
            }
        }
    }
}

pub fn fmt_bytes(n: u64) -> String {
    const KB: f64 = 1e3;
    const MB: f64 = 1e6;
    const GB: f64 = 1e9;
    const TB: f64 = 1e12;
    let (div, unit) = if n as f64 >= TB {
        (TB, "TB")
    } else if n as f64 >= GB {
        (GB, "GB")
    } else if n as f64 >= MB {
        (MB, "MB")
    } else if n as f64 >= KB {
        (KB, "KB")
    } else {
        (1.0, "B")
    };
    let v = n as f64 / div;
    let s = format!("{v:.1}");
    let s = if s.ends_with(".0") { s[..s.len() - 2].to_string() } else { s };
    format!("{s} {unit}")
}
