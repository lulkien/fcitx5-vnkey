//! Spell-check dictionary.
//!
//! The dictionary is embedded in the binary rather than read from a data
//! directory: 46 KB, no path lookup, no runtime file dependency. Every line is
//! lowercased when loading.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

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

/// The embedded dictionary, parsed once per process.
///
/// One engine exists per input context and engines are rebuilt on every config
/// change, so parsing 7.9k lines per engine would stall the daemon and hold a
/// private copy of the set per window. Share one `Arc` instead.
pub fn vietnamese() -> Arc<HashSet<String>> {
    static DICT: OnceLock<Arc<HashSet<String>>> = OnceLock::new();
    Arc::clone(DICT.get_or_init(|| Arc::new(from_lines(VIETNAMESE_DICT))))
}
