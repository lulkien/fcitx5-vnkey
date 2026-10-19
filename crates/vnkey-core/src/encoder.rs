//! Output charset encoding.
//!
//! Commits reach fcitx5 as UTF-8 (`commitString` is D-Bus type `s` and Wayland
//! text-input-v3 text), so only charsets whose output is valid UTF-8 are
//! offered. The legacy byte charsets (TCVN3, VISCII, VNI, ...) stay in the
//! generated tables for a future byte-capable frontend, but `charset_names`
//! does not list them and nothing in the addon selects them.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::charset::CHARSETS;

pub const UNICODE: &str = "Unicode";
/// Already UTF-8: the commit is a UTF-8 string, so this is the identity. (The
/// legacy `UTF-8` table is the Latin-1 rendering of those bytes, which would
/// double-encode in a string commit; `encode` never reads it.)
pub const UTF8: &str = "UTF-8";
/// Decomposed Unicode: tone marks as combining characters.
pub const UNICODE_COMBINING: &str = "Unicode tổ hợp";
/// No table: computed at encode time, see `escape_c_hex`.
pub const C_STRING_HEX: &str = "Unicode C string Hex";
pub const C_STRING_DECIMAL: &str = "Unicode C string Decimal";
pub const NCR_HEX: &str = "NCR Hex";
pub const NCR_DECIMAL: &str = "NCR Decimal";

/// What the KCM and the status menu offer: every output that is valid UTF-8.
const OFFERED: [&str; 7] = [
    UNICODE,
    UTF8,
    UNICODE_COMBINING,
    C_STRING_HEX,
    C_STRING_DECIMAL,
    NCR_HEX,
    NCR_DECIMAL,
];

fn charset_map() -> &'static HashMap<&'static str, HashMap<char, &'static str>> {
    static MAP: OnceLock<HashMap<&'static str, HashMap<char, &'static str>>> = OnceLock::new();
    MAP.get_or_init(|| {
        CHARSETS
            .iter()
            .map(|(name, table)| (*name, table.iter().copied().collect()))
            .collect()
    })
}

/// The selectable charsets, `Unicode` first.
pub fn charset_names() -> Vec<&'static str> {
    OFFERED.to_vec()
}

/// A valid C string literal body: printable ASCII kept (`\`, `"` and control
/// characters escaped), a BMP
/// character as `\uXXXX`, anything above U+FFFF as `\UXXXXXXXX`; hex uppercase.
fn escape_c_hex(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for chr in input.chars() {
        match chr {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Three-digit octal never swallows a following digit, unlike \0
            // or \x; this also keeps NUL out of the C++ std::string.
            c if c.is_ascii_control() => out.push_str(&format!("\\{:03o}", c as u32)),
            c if c.is_ascii() => out.push(c),
            c if (c as u32) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push_str(&format!("\\U{:08X}", c as u32)),
        }
    }
    out
}

fn escape_decimal(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for chr in input.chars() {
        if chr.is_ascii() {
            out.push(chr);
        } else {
            out.push_str(&(chr as u32).to_string());
        }
    }
    out
}

/// Encode NFC Vietnamese text into the named charset. Characters a table does
/// not cover, and unknown charset names, pass through unchanged.
pub fn encode(charset_name: &str, input: &str) -> String {
    match charset_name {
        UNICODE | UTF8 => return input.to_string(),
        C_STRING_HEX => return escape_c_hex(input),
        C_STRING_DECIMAL => return escape_decimal(input),
        _ => {}
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
