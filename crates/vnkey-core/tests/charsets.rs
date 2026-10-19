//! Output charsets, pinned from outside the tables.
//!
//! Expected values are not read back from `encode`/`CHARSETS`: the escape forms
//! are computed from the code point here, and the `Unicode tổ hợp` column was
//! derived with Python `unicodedata` (NFD, then breve/circumflex/horn
//! recomposed), independent of `tools/defs/charset.tsv`.

use vnkey_core::{charset_names, encode};

/// Every Vietnamese letter the tables cover, with its decomposed form.
const LETTERS: [(&str, &str); 134] = [
    ("đ", "đ"),
    ("â", "â"),
    ("ă", "ă"),
    ("ê", "ê"),
    ("ô", "ô"),
    ("ơ", "ơ"),
    ("ư", "ư"),
    ("á", "a\u{301}"),
    ("à", "a\u{300}"),
    ("ả", "a\u{309}"),
    ("ã", "a\u{303}"),
    ("ạ", "a\u{323}"),
    ("ấ", "â\u{301}"),
    ("ầ", "â\u{300}"),
    ("ẩ", "â\u{309}"),
    ("ẫ", "â\u{303}"),
    ("ậ", "â\u{323}"),
    ("ắ", "ă\u{301}"),
    ("ằ", "ă\u{300}"),
    ("ẳ", "ă\u{309}"),
    ("ẵ", "ă\u{303}"),
    ("ặ", "ă\u{323}"),
    ("é", "e\u{301}"),
    ("è", "e\u{300}"),
    ("ẻ", "e\u{309}"),
    ("ẽ", "e\u{303}"),
    ("ẹ", "e\u{323}"),
    ("ế", "ê\u{301}"),
    ("ề", "ê\u{300}"),
    ("ể", "ê\u{309}"),
    ("ễ", "ê\u{303}"),
    ("ệ", "ê\u{323}"),
    ("í", "i\u{301}"),
    ("ì", "i\u{300}"),
    ("ỉ", "i\u{309}"),
    ("ĩ", "i\u{303}"),
    ("ị", "i\u{323}"),
    ("ó", "o\u{301}"),
    ("ò", "o\u{300}"),
    ("ỏ", "o\u{309}"),
    ("õ", "o\u{303}"),
    ("ọ", "o\u{323}"),
    ("ố", "ô\u{301}"),
    ("ồ", "ô\u{300}"),
    ("ổ", "ô\u{309}"),
    ("ỗ", "ô\u{303}"),
    ("ộ", "ô\u{323}"),
    ("ớ", "ơ\u{301}"),
    ("ờ", "ơ\u{300}"),
    ("ở", "ơ\u{309}"),
    ("ỡ", "ơ\u{303}"),
    ("ợ", "ơ\u{323}"),
    ("ú", "u\u{301}"),
    ("ù", "u\u{300}"),
    ("ủ", "u\u{309}"),
    ("ũ", "u\u{303}"),
    ("ụ", "u\u{323}"),
    ("ứ", "ư\u{301}"),
    ("ừ", "ư\u{300}"),
    ("ử", "ư\u{309}"),
    ("ữ", "ư\u{303}"),
    ("ự", "ư\u{323}"),
    ("ý", "y\u{301}"),
    ("ỳ", "y\u{300}"),
    ("ỷ", "y\u{309}"),
    ("ỹ", "y\u{303}"),
    ("ỵ", "y\u{323}"),
    ("Đ", "Đ"),
    ("Â", "Â"),
    ("Ă", "Ă"),
    ("Ê", "Ê"),
    ("Ô", "Ô"),
    ("Ơ", "Ơ"),
    ("Ư", "Ư"),
    ("Á", "A\u{301}"),
    ("À", "A\u{300}"),
    ("Ả", "A\u{309}"),
    ("Ã", "A\u{303}"),
    ("Ạ", "A\u{323}"),
    ("Ấ", "Â\u{301}"),
    ("Ầ", "Â\u{300}"),
    ("Ẩ", "Â\u{309}"),
    ("Ẫ", "Â\u{303}"),
    ("Ậ", "Â\u{323}"),
    ("Ắ", "Ă\u{301}"),
    ("Ằ", "Ă\u{300}"),
    ("Ẳ", "Ă\u{309}"),
    ("Ẵ", "Ă\u{303}"),
    ("Ặ", "Ă\u{323}"),
    ("É", "E\u{301}"),
    ("È", "E\u{300}"),
    ("Ẻ", "E\u{309}"),
    ("Ẽ", "E\u{303}"),
    ("Ẹ", "E\u{323}"),
    ("Ế", "Ê\u{301}"),
    ("Ề", "Ê\u{300}"),
    ("Ể", "Ê\u{309}"),
    ("Ễ", "Ê\u{303}"),
    ("Ệ", "Ê\u{323}"),
    ("Í", "I\u{301}"),
    ("Ì", "I\u{300}"),
    ("Ỉ", "I\u{309}"),
    ("Ĩ", "I\u{303}"),
    ("Ị", "I\u{323}"),
    ("Ó", "O\u{301}"),
    ("Ò", "O\u{300}"),
    ("Ỏ", "O\u{309}"),
    ("Õ", "O\u{303}"),
    ("Ọ", "O\u{323}"),
    ("Ố", "Ô\u{301}"),
    ("Ồ", "Ô\u{300}"),
    ("Ổ", "Ô\u{309}"),
    ("Ỗ", "Ô\u{303}"),
    ("Ộ", "Ô\u{323}"),
    ("Ớ", "Ơ\u{301}"),
    ("Ờ", "Ơ\u{300}"),
    ("Ở", "Ơ\u{309}"),
    ("Ỡ", "Ơ\u{303}"),
    ("Ợ", "Ơ\u{323}"),
    ("Ú", "U\u{301}"),
    ("Ù", "U\u{300}"),
    ("Ủ", "U\u{309}"),
    ("Ũ", "U\u{303}"),
    ("Ụ", "U\u{323}"),
    ("Ứ", "Ư\u{301}"),
    ("Ừ", "Ư\u{300}"),
    ("Ử", "Ư\u{309}"),
    ("Ữ", "Ư\u{303}"),
    ("Ự", "Ư\u{323}"),
    ("Ý", "Y\u{301}"),
    ("Ỳ", "Y\u{300}"),
    ("Ỷ", "Y\u{309}"),
    ("Ỹ", "Y\u{303}"),
    ("Ỵ", "Y\u{323}"),
];

/// The NCR Hex rows that were missing, with their literal output.
const NCR_HEX_ADDED: [(&str, &str); 32] = [
    ("â", "&#xE2;"),
    ("ê", "&#xEA;"),
    ("ô", "&#xF4;"),
    ("á", "&#xE1;"),
    ("à", "&#xE0;"),
    ("ã", "&#xE3;"),
    ("é", "&#xE9;"),
    ("è", "&#xE8;"),
    ("í", "&#xED;"),
    ("ì", "&#xEC;"),
    ("ó", "&#xF3;"),
    ("ò", "&#xF2;"),
    ("õ", "&#xF5;"),
    ("ú", "&#xFA;"),
    ("ù", "&#xF9;"),
    ("ý", "&#xFD;"),
    ("Â", "&#xC2;"),
    ("Ê", "&#xCA;"),
    ("Ô", "&#xD4;"),
    ("Á", "&#xC1;"),
    ("À", "&#xC0;"),
    ("Ã", "&#xC3;"),
    ("É", "&#xC9;"),
    ("È", "&#xC8;"),
    ("Í", "&#xCD;"),
    ("Ì", "&#xCC;"),
    ("Ó", "&#xD3;"),
    ("Ò", "&#xD2;"),
    ("Õ", "&#xD5;"),
    ("Ú", "&#xDA;"),
    ("Ù", "&#xD9;"),
    ("Ý", "&#xDD;"),
];

#[test]
fn offered_charsets_are_exactly_the_deliverable_forms() {
    assert_eq!(
        charset_names(),
        [
            "Unicode",
            "UTF-8",
            "Unicode tổ hợp",
            "Unicode C string Hex",
            "Unicode C string Decimal",
            "NCR Hex",
            "NCR Decimal",
        ]
    );
}

#[test]
fn legacy_charsets_are_not_offered_but_still_encode() {
    let names = charset_names();
    for legacy in [
        "TCVN3 (ABC)",
        "VISCII",
        "VNI Windows",
        "VPS",
        "Vietware X",
        "Vietware Full",
        "BKHCM 1",
        "BKHCM 2",
        "Windows 1258 codepage",
        "VIQR",
    ] {
        assert!(!names.contains(&legacy), "{legacy} must not be offered");
        assert_ne!(encode(legacy, "đ"), "đ", "{legacy} table is kept");
    }
}

#[test]
fn every_offered_charset_encodes_every_letter_exactly() {
    for (letter, decomposed) in LETTERS {
        let cp = letter.chars().next().unwrap() as u32;
        let cases = [
            ("Unicode", letter.to_string()),
            ("UTF-8", letter.to_string()),
            ("Unicode tổ hợp", decomposed.to_string()),
            ("Unicode C string Hex", format!("\\u{cp:04X}")),
            ("Unicode C string Decimal", cp.to_string()),
            ("NCR Hex", format!("&#x{cp:X};")),
            ("NCR Decimal", format!("&#{cp};")),
        ];
        assert_eq!(cases.len(), charset_names().len());
        for (charset, expected) in cases {
            assert!(charset_names().contains(&charset), "{charset}");
            assert_eq!(encode(charset, letter), expected, "{charset}: {letter}");
        }
    }
}

#[test]
fn no_offered_charset_is_a_silent_no_op() {
    for charset in charset_names() {
        let identity = matches!(charset, "Unicode" | "UTF-8");
        // ệ, not đ: "Unicode tổ hợp" legitimately leaves đ alone.
        assert_eq!(encode(charset, "ệ") == "ệ", identity, "{charset}");
    }
}

#[test]
fn ncr_hex_covers_the_previously_missing_letters() {
    for (letter, expected) in NCR_HEX_ADDED {
        assert_eq!(encode("NCR Hex", letter), expected, "{letter}");
    }
}

#[test]
fn c_string_hex_is_a_valid_c_literal() {
    let hex = "Unicode C string Hex";
    assert_eq!(encode(hex, "đ"), "\\u0111");
    assert_eq!(encode(hex, "\\"), "\\\\");
    assert_eq!(encode(hex, "\""), "\\\"");
    assert_eq!(encode(hex, "😀"), "\\U0001F600");
    assert_eq!(encode(hex, "say \"đ\\n\""), "say \\\"\\u0111\\\\n\\\"");

    // Decimal is a plain decimal code point; ASCII, backslash included, stays.
    let dec = "Unicode C string Decimal";
    assert_eq!(encode(dec, "😀"), "128512");
    assert_eq!(encode(dec, "a\\\"đ"), "a\\\"273");
}

/// Legacy tables are unreachable from the UI now; pin the rows fixed in
/// round 4 directly.
#[test]
fn fixed_legacy_rows_are_pinned() {
    // Standard VISCII (RFC 1456), not the lowercase code.
    for (letter, byte) in [
        ("Ẫ", '\u{06}'),
        ("Ẳ", '\u{02}'),
        ("Ẵ", '\u{05}'),
        ("Õ", '\u{A0}'),
        ("Ỷ", '\u{14}'),
        ("Ỹ", '\u{19}'),
        ("Ỵ", '\u{1E}'),
    ] {
        assert_eq!(encode("VISCII", letter), byte.to_string(), "{letter}");
    }
    // Their lowercase partners keep their own codes.
    assert_eq!(encode("VISCII", "ẫẳẵõỷỹỵ"), "çÆÇõÖÛÜ");
    // Windows 1258 Ứ: U+00DD U+00EC, no stray '='.
    assert_eq!(encode("Windows 1258 codepage", "Ứ"), "\u{DD}\u{EC}");
}
