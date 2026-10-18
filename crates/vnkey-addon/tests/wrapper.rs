//! Smoke tests for the addon layer.
//!
//! These pin the behaviour of the addon layer.
//! Authoritative equivalence against upstream comes from the differential
//! harness (see `tools/`), which drives both implementations through the same
//! key streams; these tests are the local regression net.

use std::collections::HashSet;

use vnkey_addon::{VnKeyEngine, MacroTable, Options, FCITX_BACKSPACE, FCITX_LOCK_MASK, FCITX_TAB};

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
    let mut e = VnKeyEngine::new("Telex 2", HashSet::new(), macros, options).expect("Telex 2");

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
fn charset_encoding_is_applied_to_the_preedit() {
    let options = Options {
        output_charset: "TCVN3 (ABC)".to_string(),
        ..Options::default()
    };
    let mut e = engine_with(options);
    type_str(&mut e, "ddaw");
    // TCVN3 has no combining forms: đ -> ®, ă -> ¨ (from the generated table).
    assert_eq!(e.pull_preedit(), "®¨");
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
        assert!(!e.process_key_event(ch as u32, 0), "key must not be swallowed");
    }
    assert_eq!(e.pull_preedit(), "");
    assert_eq!(e.pull_commit(), "");
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
