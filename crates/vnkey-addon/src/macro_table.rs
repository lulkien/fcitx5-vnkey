//! Macro table.
//!
//! Quirk preserved from upstream: `has_key`/`get_text` lowercase the query,
//! while `has_prefix` does not (it compares the stored keys verbatim), so macro
//! keys are expected to be stored lowercase and prefix matching is
//! case-sensitive.

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct MacroTable {
    entries: HashMap<String, String>,
}

impl MacroTable {
    pub fn new() -> MacroTable {
        MacroTable::default()
    }

    /// Keys are stored verbatim, as `NewMacroTable` does.
    pub fn from_pairs<I, K, V>(pairs: I) -> MacroTable
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        MacroTable {
            entries: pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// Present and non-empty, matched against the lowercased key.
    pub fn has_key(&self, key: &str) -> bool {
        self.entries
            .get(&key.to_lowercase())
            .is_some_and(|value| !value.is_empty())
    }

    pub fn get_text(&self, key: &str) -> String {
        self.entries
            .get(&key.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Upstream's `HasPrefix`: exact lookup without case folding, then a
    /// case-sensitive prefix scan over the stored keys.
    pub fn has_prefix(&self, key: &str) -> bool {
        if self.entries.get(key).is_some_and(|v| !v.is_empty()) {
            return true;
        }
        self.entries.keys().any(|stored| stored.starts_with(key))
    }
}
