//! Character tables.
//!
//! The layout is load-bearing, not cosmetic: a `Tone` discriminant indexes a row,
//! and a `Mark` discriminant indexes a mark row. Upstream encodes the same thing
//! as one flat `Vowels` string plus per-character mark strings.

/// All Vietnamese vowels grouped by base character, in upstream order:
/// `a ă â e ê i o ô ơ u ư y`, each row ordered by `Tone`.
pub const VOWEL_ROWS: [[char; 6]; 12] = [
    ['a', 'à', 'á', 'ả', 'ã', 'ạ'],
    ['ă', 'ằ', 'ắ', 'ẳ', 'ẵ', 'ặ'],
    ['â', 'ầ', 'ấ', 'ẩ', 'ẫ', 'ậ'],
    ['e', 'è', 'é', 'ẻ', 'ẽ', 'ẹ'],
    ['ê', 'ề', 'ế', 'ể', 'ễ', 'ệ'],
    ['i', 'ì', 'í', 'ỉ', 'ĩ', 'ị'],
    ['o', 'ò', 'ó', 'ỏ', 'õ', 'ọ'],
    ['ô', 'ồ', 'ố', 'ổ', 'ỗ', 'ộ'],
    ['ơ', 'ờ', 'ớ', 'ở', 'ỡ', 'ợ'],
    ['u', 'ù', 'ú', 'ủ', 'ũ', 'ụ'],
    ['ư', 'ừ', 'ứ', 'ử', 'ữ', 'ự'],
    ['y', 'ỳ', 'ý', 'ỷ', 'ỹ', 'ỵ'],
];

/// Mark slot rows: index = `Mark` discriminant, `_` = no character.
/// Every member of a family maps to the same row.
pub fn mark_row(chr: char) -> Option<&'static [char; 5]> {
    const A: [char; 5] = ['a', 'â', 'ă', '_', '_'];
    const E: [char; 5] = ['e', 'ê', '_', '_', '_'];
    const O: [char; 5] = ['o', 'ô', '_', 'ơ', '_'];
    const U: [char; 5] = ['u', '_', '_', 'ư', '_'];
    const D: [char; 5] = ['d', '_', '_', '_', 'đ'];
    match chr {
        'a' | 'â' | 'ă' => Some(&A),
        'e' | 'ê' => Some(&E),
        'o' | 'ô' | 'ơ' => Some(&O),
        'u' | 'ư' => Some(&U),
        'd' | 'đ' => Some(&D),
        _ => None,
    }
}

pub const PUNCTUATION_MARKS: [char; 33] = [
    ',', ';', ':', '.', '"', '\'', '!', '?', ' ', '<', '>', '=', '+', '-', '*', '/', '\\', '_', '~',
    '`', '@', '#', '$', '%', '^', '&', '(', ')', '{', '}', '[', ']', '|',
];

/// Simple (single code point) lowercase, matching Go's `unicode.ToLower`, which
/// does not do full case folding like Rust's `char::to_lowercase`.
pub fn simple_lower(chr: char) -> char {
    chr.to_lowercase().next().unwrap_or(chr)
}

/// Simple (single code point) uppercase, matching Go's `unicode.ToUpper`.
pub fn simple_upper(chr: char) -> char {
    chr.to_uppercase().next().unwrap_or(chr)
}
