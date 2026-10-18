//! Output charset encoding.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::charset::CHARSETS;

pub const UNICODE: &str = "Unicode";

fn charset_map() -> &'static HashMap<&'static str, HashMap<char, &'static str>> {
    static MAP: OnceLock<HashMap<&'static str, HashMap<char, &'static str>>> = OnceLock::new();
    MAP.get_or_init(|| {
        CHARSETS
            .iter()
            .map(|(name, table)| (*name, table.iter().copied().collect()))
            .collect()
    })
}

/// Charset names, `Unicode` first.
pub fn charset_names() -> Vec<&'static str> {
    let mut names = vec![UNICODE];
    names.extend(CHARSETS.iter().map(|(name, _)| *name));
    names
}

/// Encode NFC Vietnamese text into the named charset. Unknown characters and
/// unknown charset names pass through unchanged.
pub fn encode(charset_name: &str, input: &str) -> String {
    if charset_name == UNICODE {
        return input.to_string();
    }
    let Some(table) = charset_map().get(charset_name) else {
        return input.to_string();
    };
    let mut out = String::with_capacity(input.len());
    for chr in input.chars() {
        match table.get(&chr) {
            Some(replacement) => out.push_str(replacement),
            None => out.push(chr),
        }
    }
    out
}
