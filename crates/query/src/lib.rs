mod chip;
mod date;
mod filter;
mod size;

pub use chip::{Chip, ChipKind};
pub use date::DateSpec;
pub use filter::{Filter, Parsed};
pub use size::SizeSpec;

/// Parse a plain-English query into filter chips plus free-text terms.
/// Words the parser does not understand stay searchable as text; nothing is dropped.
pub fn parse(input: &str) -> Parsed {
    filter::parse(input)
}
