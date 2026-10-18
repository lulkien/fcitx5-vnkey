//! Spell-check dictionary.
//!
//! The dictionary is embedded in the binary rather than read from a data
//! directory: 46 KB, no path lookup, no runtime file dependency. Every line is
//! lowercased when loading.

use std::collections::HashSet;

/// Vietnamese syllable dictionary. Provenance is not established; treat it as
/// third-party data until that is verified (see NOTICE).
pub const VIETNAMESE_DICT: &str = include_str!("../data/vietnamese.cm.dict");

pub fn from_lines(text: &str) -> HashSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_lowercase())
        .collect()
}

/// The default dictionary, built once per call (≈7.9k entries).
pub fn vietnamese() -> HashSet<String> {
    from_lines(VIETNAMESE_DICT)
}
