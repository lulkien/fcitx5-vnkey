//! Smoke tests for the addon layer.
//!
//! These pin the behaviour of the addon layer.
//! Authoritative equivalence against upstream comes from the differential
//! harness (see `tools/`), which drives both implementations through the same
//! key streams; these tests are the local regression net.

use std::collections::HashSet;
use std::sync::Arc;

use vnkey_addon::{MacroTable, Options, VnKeyEngine, FCITX_BACKSPACE, FCITX_LOCK_MASK, FCITX_TAB};

fn engine_with(options: Options) -> VnKeyEngine {
    VnKeyEngine::new(
        "Telex 2",
        vnkey_addon::dictionary::vietnamese(),
        MacroTable::new(),
        options,
    )
    .expect("Telex 2")
}

fn engine() -> VnKeyEngine {
    engine_with(Options::default())
}

fn type_str(engine: &mut VnKeyEngine, text: &str) {
    for ch in text.chars() {
        engine.process_key_event(ch as u32, 0);
    }
}

#[test]
fn types_and_commits_on_space() {
    let mut e = engine();
    type_str(&mut e, "vieetj");
    assert_eq!(e.pull_preedit(), "việt");

    // Space is a word break: it is committed along with the word.
    let consumed = e.process_key_event(' ' as u32, 0);
    assert!(consumed);
    assert_eq!(e.pull_commit(), "việt ");
    assert_eq!(e.pull_preedit(), "");
}

#[test]
fn backspace_replays_the_word() {
    let mut e = engine();
    type_str(&mut e, "vieetj");
    assert_eq!(e.pull_preedit(), "việt");

    assert!(e.process_key_event(FCITX_BACKSPACE, 0));
    assert_eq!(e.pull_preedit(), "việ");

    for _ in 0..10 {
        e.process_key_event(FCITX_BACKSPACE, 0);
    }
    assert_eq!(e.pull_preedit(), "");
}

#[test]
fn non_vietnamese_word_stays_raw() {
    let mut e = engine();
    type_str(&mut e, "hello");
    assert_eq!(e.pull_preedit(), "hello");
}

#[test]
fn caps_lock_arrives_as_an_uppercase_key() {
    // fcitx hands us the *key symbol*: with Caps Lock on, 'd' arrives as 'D',
    // so the engine only swaps bracket pairs here. Upstream behaves the same.
    let mut e = engine();
    for ch in "dd".chars() {
        e.process_key_event(ch as u32, FCITX_LOCK_MASK);
    }
    assert_eq!(e.pull_preedit(), "đ");

    let mut e = engine();
    for ch in "DD".chars() {
        e.process_key_event(ch as u32, FCITX_LOCK_MASK);
    }
    assert_eq!(e.pull_preedit(), "Đ");
}

#[test]
fn macro_expands_on_tab() {
    let options = Options {
        macro_enabled: true,
        ..Options::default()
    };
    let macros = MacroTable::from_pairs([("vn", "Việt Nam")]);
    let mut e =
        VnKeyEngine::new("Telex 2", Arc::new(HashSet::new()), macros, options).expect("Telex 2");

    type_str(&mut e, "vn");
    let consumed = e.process_key_event(FCITX_TAB, 0);
    assert!(consumed);
    assert_eq!(e.pull_commit(), "Việt Nam");
}

#[test]
fn macro_capitalisation_follows_the_keystrokes() {
    use vnkey_addon::{determine_macro_case, MacroCase};
    assert_eq!(determine_macro_case("vn"), MacroCase::AllSmall);
    assert_eq!(determine_macro_case("VN"), MacroCase::AllCapital);
    assert_eq!(determine_macro_case("Vn"), MacroCase::NoChange);
    assert_eq!(determine_macro_case(""), MacroCase::NoChange);
}

#[test]
fn charset_applies_to_the_commit_not_the_preedit() {
    // Unicode: nothing changes.
    let mut e = engine();
    type_str(&mut e, "ddaw");
    assert_eq!(e.pull_preedit(), "đă");
    e.process_key_event(' ' as u32, 0);
    assert_eq!(e.pull_commit(), "đă ");

    // Escape charsets: the preedit stays readable, the commit is escaped.
    let cases = [
        ("Unicode C string Hex", "\\u0111\\u0103 "),
        ("NCR Decimal", "&#273;&#259; "),
        ("NCR Hex", "&#x111;&#x103; "),
    ];
    for (charset, text) in cases {
        let mut e = engine_with(Options {
            output_charset: charset.to_string(),
            ..Options::default()
        });
        type_str(&mut e, "ddaw");
        assert_eq!(e.pull_preedit(), "đă", "{charset}: preedit stays Unicode");
        e.process_key_event(' ' as u32, 0);
        assert_eq!(e.pull_commit(), text, "{charset}");
    }
}

#[test]
fn charset_applies_to_macro_and_flush_commits() {
    let mut e = VnKeyEngine::new(
        "Telex 2",
        vnkey_addon::dictionary::vietnamese(),
        MacroTable::from_pairs([("vn", "Việt Nam")]),
        Options {
            output_charset: "NCR Decimal".to_string(),
            macro_enabled: true,
            ..Options::default()
        },
    )
    .expect("Telex 2");
    type_str(&mut e, "vn");
    e.process_key_event(FCITX_TAB, 0);
    // The expansion is encoded once.
    assert_eq!(e.pull_commit(), "Vi&#7879;t Nam");

    // An explicit commit is encoded too.
    type_str(&mut e, "ddaw");
    e.commit_preedit();
    assert_eq!(e.pull_commit(), "&#273;&#259;");
}

/// The facade must not call `process_key_event` for the restore hotkey when
/// there is nothing to restore (a macro-enabled engine would commit the space
/// and eat the key); `restore_key_stroke() == false` is what tells it so.
/// Covers the engine side of that decision, not the C++ branch itself.
#[test]
fn restore_key_is_not_ours_without_a_composition() {
    for macro_enabled in [true, false] {
        let mut e = engine_with(Options {
            macro_enabled,
            ..Options::default()
        });
        assert!(!e.restore_key_stroke(), "vietnamese, macro {macro_enabled}");
        assert!(e.pull_commit().is_empty());
        assert_eq!(e.pull_preedit(), "");
    }

    let mut e = engine();
    e.set_vietnamese_mode(false);
    assert!(!e.restore_key_stroke(), "english");
    assert!(e.pull_commit().is_empty());

    // With a composition it is ours.
    let mut e = engine();
    type_str(&mut e, "vieetj");
    assert!(e.restore_key_stroke());
    assert_eq!(e.pull_preedit(), "vieetj");
}

/// Config-driven switch to English: the facade commits the word first
/// (`commitBuffer`), then applies the mode. Afterwards nothing is left to
/// restore, so the next Shift+Space reaches the application.
#[test]
fn leaving_vietnamese_mode_through_config_leaves_no_stale_preedit() {
    let mut e = engine();
    type_str(&mut e, "vie");
    e.commit_preedit();
    e.set_vietnamese_mode(false);
    assert_eq!(e.pull_commit(), "vie");
    assert_eq!(e.pull_preedit(), "");
    assert!(!e.restore_key_stroke());
}

#[test]
fn control_modifier_passes_keys_through() {
    let mut e = engine();
    type_str(&mut e, "vieetj");
    let consumed = e.process_key_event('a' as u32, vnkey_addon::FCITX_CONTROL_MASK);
    assert!(!consumed, "Ctrl+A must not be swallowed");
}

#[test]
fn english_mode_passes_everything_through() {
    let mut e = engine();
    e.set_vietnamese_mode(false);
    assert!(!e.vietnamese_mode());

    for ch in "vieetj".chars() {
        assert!(
            !e.process_key_event(ch as u32, 0),
            "key must not be swallowed"
        );
    }
    assert_eq!(e.pull_preedit(), "");
    assert!(e.pull_commit().is_empty());
}

#[test]
fn switching_to_english_commits_the_pending_word() {
    let mut e = engine();
    type_str(&mut e, "vieetj");
    assert_eq!(e.pull_preedit(), "việt");

    e.set_vietnamese_mode(false);
    // The half-typed word must not vanish when the mode flips.
    let consumed = e.process_key_event(' ' as u32, 0);
    assert!(!consumed);
    assert_eq!(e.pull_commit(), "việt");
    assert_eq!(e.pull_preedit(), "");
}

#[test]
fn switching_back_resumes_composition() {
    let mut e = engine();
    e.set_vietnamese_mode(false);
    type_str(&mut e, "vieetj");
    assert_eq!(e.pull_preedit(), "");

    e.set_vietnamese_mode(true);
    type_str(&mut e, "vieetj");
    assert_eq!(e.pull_preedit(), "việt");
}

#[test]
fn toggle_flips_and_reports() {
    let mut e = engine();
    assert!(e.vietnamese_mode());
    assert!(!e.toggle_vietnamese_mode());
    assert!(e.toggle_vietnamese_mode());
}

#[test]
fn embedded_dictionary_is_parsed_once_and_shared() {
    // Engines are rebuilt per config change and per input context; the set must
    // not be re-parsed or copied for each of them.
    let a = vnkey_addon::dictionary::vietnamese();
    let b = vnkey_addon::dictionary::vietnamese();
    assert!(Arc::ptr_eq(&a, &b));
    assert!(!a.is_empty());
}

#[test]
fn restore_key_stroke_rewrites_the_word_in_place() {
    let mut e = engine();
    type_str(&mut e, "vieetj");
    assert_eq!(e.pull_preedit(), "việt");

    // Applied eagerly: the facade swallows the hotkey and re-renders this.
    e.restore_key_stroke();
    assert_eq!(e.pull_preedit(), "vieetj");

    // The keystroke after the hotkey must still land.
    e.process_key_event('x' as u32, 0);
    assert_eq!(e.pull_preedit(), "vieetjx");
}
