//! Character predicates and mark/tone arithmetic.

use crate::rules::{Mark, Tone};
use crate::tables::{mark_row, simple_lower, PUNCTUATION_MARKS, VOWEL_ROWS};

pub fn is_space(key: char) -> bool {
    key == ' '
}

pub fn is_punctuation_mark(key: char) -> bool {
    PUNCTUATION_MARKS.contains(&key)
}

pub fn is_word_break_symbol(key: char) -> bool {
    is_punctuation_mark(key) || key.is_ascii_digit()
}

/// Flat index into the upstream vowel table: `row * 6 + tone`.
/// `None` for anything that is not a Vietnamese vowel (toned or not).
pub fn find_vowel_position(chr: char) -> Option<usize> {
    VOWEL_ROWS
        .iter()
        .position(|row| row.contains(&chr))
        .map(|row| row * 6 + VOWEL_ROWS[row].iter().position(|c| *c == chr).unwrap())
}

pub fn is_vowel(chr: char) -> bool {
    find_vowel_position(chr).is_some()
}

pub fn find_tone_from_char(chr: char) -> Tone {
    match find_vowel_position(chr) {
        Some(pos) => Tone::from_u8((pos % 6) as u8),
        None => Tone::None,
    }
}

/// Replace the tone of a vowel, keeping its mark. `tone == Tone::None` strips it.
pub fn add_tone_to_char(chr: char, tone: u8) -> char {
    match find_vowel_position(chr) {
        Some(pos) => VOWEL_ROWS[pos / 6].get(tone as usize).copied().unwrap_or(chr),
        None => chr,
    }
}

/// Mark family members (excluding empty slots), e.g. `ă` -> `[a, â, ă]`.
pub fn mark_family(chr: char) -> Vec<char> {
    match mark_row(chr) {
        Some(row) => row.iter().copied().filter(|c| *c != '_').collect(),
        None => Vec::new(),
    }
}

pub fn find_mark_position(chr: char) -> Option<usize> {
    mark_row(chr)?.iter().position(|c| *c == chr)
}

pub fn find_mark_from_char(chr: char) -> Option<Mark> {
    find_mark_position(chr).map(|pos| Mark::from_u8(pos as u8))
}

/// Apply a mark slot to a toneless character; `_` slots leave it unchanged.
pub fn add_mark_to_toneless_char(chr: char, mark: u8) -> char {
    match mark_row(chr) {
        Some(row) => match row.get(mark as usize) {
            Some(&candidate) if candidate != '_' => candidate,
            _ => chr,
        },
        None => chr,
    }
}

/// Apply a mark, preserving the existing tone.
pub fn add_mark_to_char(chr: char, mark: u8) -> char {
    let tone = find_tone_from_char(chr);
    let bare = add_tone_to_char(chr, 0);
    let marked = add_mark_to_toneless_char(bare, mark);
    add_tone_to_char(marked, tone as u8)
}

pub fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic()
}

pub fn in_key_list(keys: &[char], key: char) -> bool {
    keys.contains(&key)
}

/// True when the character carries a Vietnamese tone or a Vietnamese mark.
pub fn is_vietnamese_rune(lower_key: char) -> bool {
    if find_tone_from_char(lower_key) != Tone::None {
        return true;
    }
    lower_key != add_mark_to_toneless_char(lower_key, 0)
}

pub fn has_any_vietnamese_rune(word: &str) -> bool {
    word.chars().any(|c| is_vietnamese_rune(simple_lower(c)))
}

pub fn has_any_vietnamese_vowel(word: &str) -> bool {
    word.chars().any(|c| is_vowel(simple_lower(c)))
}

/// Keys the engine is willing to process beyond plain ASCII letters.
pub fn can_process_key(lower_key: char, effect_keys: &[char]) -> bool {
    if is_alpha(lower_key) || in_key_list(effect_keys, lower_key) {
        return true;
    }
    if is_word_break_symbol(lower_key) {
        return false;
    }
    is_vietnamese_rune(lower_key)
}
