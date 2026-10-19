//! Deterministic randomized smoke test for the addon engine.
//!
//! The release profile builds with `panic = "abort"`, so a panic anywhere in
//! key handling takes down the whole fcitx5 daemon rather than one keypress.
//! The unit tests spell out the sequences we expect; this one asserts the
//! engine is total over arbitrary input: every layout, every output charset,
//! both modes, with and without macros, plus the buffer lifecycle calls the
//! facade makes (commit / reset / restore / option reload).

use vnkey_addon::{MacroTable, Options, VnKeyEngine};

/// Small deterministic LCG: a failure reproduces on the next run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0 >> 33
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() as usize) % items.len()]
    }
}

const MODIFIERS: [u32; 5] = [
    0,
    0,
    0,
    vnkey_addon::FCITX_CONTROL_MASK,
    vnkey_addon::FCITX_LOCK_MASK,
];

fn key_pool() -> Vec<u32> {
    let mut keys: Vec<u32> = Vec::new();
    for ch in
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789[]{}()'`~?^+* .,;:-_=/\\|@#$%&!"
            .chars()
    {
        keys.push(ch as u32);
    }
    for ch in "ăâêôơưđĂÂÊÔƠƯĐáàảãạếệ".chars() {
        keys.push(ch as u32);
    }
    // Controls and out-of-range code points the X server can actually deliver.
    keys.extend([
        vnkey_addon::FCITX_BACKSPACE,
        vnkey_addon::FCITX_TAB,
        vnkey_addon::FCITX_SPACE,
        0xff0d,   // Return
        0xff1b,   // Escape
        0x007f,   // Delete
        0x10ffff, // highest scalar value
        0xd800,   // lone surrogate: not a `char`
        0x0000,   // NUL
    ]);
    keys
}

fn run_stream(method: &str, charset: &str, macro_enabled: bool, seed: u64, keys: &[u32]) {
    // The live options: lifecycle steps change one field at a time and never
    // the charset, so the stream really runs in the charset under test.
    let mut options = Options {
        output_charset: charset.to_string(),
        macro_enabled,
        ..Options::default()
    };
    let macros = MacroTable::from_pairs([("vn", "Việt Nam"), ("ko", ""), ("", "empty")]);
    let mut engine = VnKeyEngine::new(
        method,
        vnkey_addon::dictionary::vietnamese(),
        macros,
        options.clone(),
    )
    .unwrap_or_else(|| panic!("unknown input method {method}"));

    let mut rng = Rng(seed);
    for i in 0..400 {
        // Every so often exercise the lifecycle calls the facade makes between
        // keys: these must not panic on any intermediate state either.
        match i % 97 {
            0 => engine.commit_preedit(),
            1 => engine.reset(),
            2 => {
                engine.restore_key_stroke();
            }
            3 => {
                options.spell_check_with_dicts = !options.spell_check_with_dicts;
                engine.set_options(options.clone());
            }
            4 => {
                options.macro_enabled = !options.macro_enabled;
                engine.set_options(options.clone());
            }
            // English mode for one key, then straight back.
            5 => engine.set_vietnamese_mode(false),
            6 => engine.set_vietnamese_mode(true),
            _ => {}
        }
        let key = *rng.pick(keys);
        let state = *rng.pick(&MODIFIERS);
        engine.process_key_event(key, state);

        // The commit buffer must always be drainable and reset on read.
        let _ = engine.pull_commit();
        assert!(engine.pull_commit().is_empty());
        let _ = engine.pull_preedit();
    }
}

#[test]
fn every_layout_and_charset_survives_random_key_streams() {
    let keys = key_pool();
    for (m, method) in vnkey_core::input_method_names().iter().enumerate() {
        for (c, charset) in vnkey_core::charset_names().iter().enumerate() {
            for macro_enabled in [false, true] {
                let seed = ((m as u64 + 1) * 0x9e37_79b9) ^ (c as u64 + 1) ^ macro_enabled as u64;
                run_stream(method, charset, macro_enabled, seed, &keys);
            }
        }
    }
}

fn engine_for(charset: &str) -> VnKeyEngine {
    VnKeyEngine::new(
        "Telex 2",
        vnkey_addon::dictionary::vietnamese(),
        MacroTable::new(),
        Options {
            output_charset: charset.to_string(),
            ..Options::default()
        },
    )
    .expect("Telex 2")
}

fn type_str(engine: &mut VnKeyEngine, text: &str) {
    for ch in text.chars() {
        engine.process_key_event(ch as u32, 0);
    }
}

/// (charset, commit for "ddaw" + space, commit for "Vieetj" + space), written
/// out by hand so a wrong table fails here. ệ is U+1EC7 = 7879; "Unicode tổ
/// hợp" keeps ê precomposed and splits the dot below off.
const PINNED: [(&str, &str, &str); 7] = [
    ("Unicode", "đă ", "Việt "),
    ("UTF-8", "đă ", "Việt "),
    ("Unicode tổ hợp", "đă ", "Vi\u{ea}\u{323}t "),
    ("Unicode C string Hex", "\\u0111\\u0103 ", "Vi\\u1EC7t "),
    ("Unicode C string Decimal", "273259 ", "Vi7879t "),
    ("NCR Hex", "&#x111;&#x103; ", "Vi&#x1EC7;t "),
    ("NCR Decimal", "&#273;&#259; ", "Vi&#7879;t "),
];

#[test]
fn every_offered_charset_commits_the_pinned_text() {
    let mut offered = vnkey_core::charset_names();
    offered.sort_unstable();
    let mut pinned: Vec<_> = PINNED.iter().map(|row| row.0).collect();
    pinned.sort_unstable();
    assert_eq!(offered, pinned, "a charset is offered but not pinned");

    for (charset, dd_aw, viet) in PINNED {
        for (keys, word, expected) in [("ddaw", "đă", dd_aw), ("Vieetj", "Việt", viet)] {
            let mut e = engine_for(charset);
            type_str(&mut e, keys);
            // The preedit is readable Unicode in every charset.
            assert_eq!(e.pull_preedit(), word, "{charset}: preedit");
            e.process_key_event(' ' as u32, 0);
            assert_eq!(e.pull_commit(), expected, "{charset}: commit of {word}");
            assert_eq!(e.pull_preedit(), "", "{charset}: preedit after commit");
        }
    }
}

#[test]
fn lifecycle_option_changes_keep_the_charset_and_the_word() {
    let charset = "Unicode C string Hex";
    let mut e = engine_for(charset);
    type_str(&mut e, "vieet");
    let before = e.pull_preedit().to_string();

    // What the facade does for a spell-check toggle: same options, one flag.
    e.set_options(Options {
        output_charset: charset.to_string(),
        spell_check_with_dicts: false,
        ..Options::default()
    });
    assert_eq!(e.pull_preedit(), before);
    e.process_key_event('j' as u32, 0);
    assert_eq!(e.pull_preedit(), "việt");
}
