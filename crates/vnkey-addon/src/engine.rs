//! Per-input-context engine.
//!
//! Holds the preedit / commit buffers and the policy that decides when to fall
//! back to raw keystrokes. All of it is plain Rust; the only C++ in the project
//! is the fcitx5 facade that calls into it.

use std::collections::HashSet;
use std::sync::Arc;

use vnkey_core::utils::{has_any_vietnamese_rune, has_any_vietnamese_vowel, is_word_break_symbol};
use vnkey_core::{encode, Engine as Preeditor, Mode};

use crate::macro_table::MacroTable;
use crate::options::Options;

pub const FCITX_SHIFT_MASK: u32 = 1 << 0;
pub const FCITX_LOCK_MASK: u32 = 1 << 1;
pub const FCITX_CONTROL_MASK: u32 = 1 << 2;
pub const FCITX_MOD1_MASK: u32 = 1 << 3;
pub const FCITX_FORWARD_MASK: u32 = 1 << 25;
pub const FCITX_IGNORED_MASK: u32 = FCITX_FORWARD_MASK;
pub const FCITX_SUPER_MASK: u32 = 1 << 26;
pub const FCITX_HYPER_MASK: u32 = 1 << 27;
pub const FCITX_META_MASK: u32 = 1 << 28;

pub const FCITX_BACKSPACE: u32 = 0xff08;
pub const FCITX_SPACE: u32 = 0x020;
pub const FCITX_TAB: u32 = 0xff09;

/// Case policy for macro expansion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MacroCase {
    AllSmall,
    AllCapital,
    NoChange,
}

/// Decide how a macro should be capitalised from the keystrokes typed.
pub fn determine_macro_case(text: &str) -> MacroCase {
    let chars: Vec<char> = text.chars().collect();
    let Some(&first) = chars.first() else {
        return MacroCase::NoChange;
    };
    if first.is_lowercase() {
        return MacroCase::AllSmall;
    }
    for &c in chars.iter().skip(1) {
        if c.is_lowercase() {
            return MacroCase::NoChange;
        }
        if is_word_break_symbol(c) {
            return MacroCase::NoChange;
        }
    }
    MacroCase::AllCapital
}

pub struct VnKeyEngine {
    preeditor: Preeditor,
    macro_table: MacroTable,
    /// Shared, parsed once per process; never owned per engine.
    dictionary: Arc<HashSet<String>>,
    options: Options,
    /// Global Vietnamese/English switch. Off = every key passes through.
    vietnamese_mode: bool,
    #[allow(dead_code)]
    last_key_with_shift: bool,
    preedit_text: String,
    commit_text: String,
}

impl VnKeyEngine {
    /// Build an engine for the named input method. `None` for unknown names.
    pub fn new(
        input_method: &str,
        dictionary: Arc<HashSet<String>>,
        macro_table: MacroTable,
        options: Options,
    ) -> Option<VnKeyEngine> {
        let input_method = vnkey_core::parse_input_method(input_method)?;
        Some(VnKeyEngine {
            preeditor: Preeditor::new(input_method, options.flags()),
            macro_table,
            dictionary,
            options,
            vietnamese_mode: true,
            last_key_with_shift: false,
            preedit_text: String::new(),
            commit_text: String::new(),
        })
    }

    // =====================================================================
    // Public surface used by the fcitx5 facade
    // =====================================================================

    /// Process one key. Returns `true` when the key must be swallowed.
    pub fn process_key_event(&mut self, key_val: u32, state: u32) -> bool {
        // Upstream defers this so it runs after every return path.
        let consumed = self.process_key_event_inner(key_val, state);
        self.update_last_key_with_shift(key_val, state);
        consumed
    }

    pub fn set_options(&mut self, options: Options) {
        self.preeditor.set_flags(options.flags());
        self.options = options;
    }

    /// Restore the raw keystrokes of the word currently being composed.
    ///
    /// Applied eagerly, in place: the facade swallows the hotkey itself, so
    /// deferring the restore to the next key event would swallow that next
    /// keystroke along with it.
    ///
    /// Returns `false`, touching nothing, when there is no composition: the
    /// key is then an ordinary one and must reach the application. That has
    /// to be decided here, before any word-break handling, which would
    /// otherwise consume it (a macro-enabled engine commits the space).
    pub fn restore_key_stroke(&mut self) -> bool {
        if self.preedit_text.is_empty() {
            return false;
        }
        let old_text = self.get_preedit_string();
        self.preeditor
            .restore_last_word(!has_any_vietnamese_rune(&old_text));
        let restored = self.get_preedit_string();
        self.update_preedit(&restored);
        true
    }

    /// Vietnamese (true) or English (false) mode.
    ///
    /// In English mode nothing is composed: keys are passed through untouched,
    /// and anything already being composed is committed first so the word is
    /// not lost.
    pub fn set_vietnamese_mode(&mut self, enabled: bool) {
        self.vietnamese_mode = enabled;
    }

    pub fn vietnamese_mode(&self) -> bool {
        self.vietnamese_mode
    }

    pub fn toggle_vietnamese_mode(&mut self) -> bool {
        self.vietnamese_mode = !self.vietnamese_mode;
        self.vietnamese_mode
    }

    pub fn pull_preedit(&self) -> &str {
        &self.preedit_text
    }

    /// Returns the pending commit string and clears it.
    pub fn pull_commit(&mut self) -> String {
        std::mem::take(&mut self.commit_text)
    }

    /// Commit whatever is in the preedit and start over.
    pub fn commit_preedit(&mut self) {
        let preedit = self.get_preedit_string();
        self.commit_preedit_and_reset(&preedit);
    }

    pub fn reset(&mut self) {
        self.commit_preedit_and_reset("");
    }

    // =====================================================================
    // Key handling
    // =====================================================================

    fn process_key_event_inner(&mut self, key_val: u32, state: u32) -> bool {
        if !self.vietnamese_mode {
            // Commit a half-typed word instead of dropping it, then let the
            // application handle the key itself.
            if !self.preedit_text.is_empty() {
                self.commit_preedit();
            }
            return false;
        }

        let raw_key_len = self.get_raw_key_len();
        let key_rune = rune_of(key_val);
        let old_text = self.get_preedit_string();

        if !self.preeditor.can_process_key(key_rune)
            && raw_key_len == 0
            && !self.options.macro_enabled
        {
            // Don't touch special characters when nothing has been typed yet;
            // works around Chrome's address bar and Google Sheets.
            return false;
        }

        if key_val == FCITX_BACKSPACE {
            if self.rune_count() == 1 {
                self.commit_preedit_and_reset("");
                return true;
            }
            if raw_key_len > 0 {
                self.preeditor.remove_last_char(true);
                let preedit = self.get_preedit_string();
                self.update_preedit(&preedit);
                return true;
            }
            return false;
        }

        if key_val == FCITX_TAB {
            let (has_macro, macro_text) = self.get_macro_text();
            if has_macro {
                self.commit_preedit_and_reset(&macro_text);
            } else {
                let composed = self.get_composed_string(&old_text);
                self.commit_preedit_and_reset(&composed);
                return false;
            }
            return true;
        }

        let (new_text, is_word_break_rune) = self.get_commit_text(key_val, state);
        let is_printable_key = self.is_printable_key(state, key_val);
        if is_word_break_rune {
            self.commit_preedit_and_reset(&new_text);
            return is_printable_key;
        }
        self.update_preedit(&new_text);
        is_printable_key
    }

    fn get_commit_text(&mut self, key_val: u32, state: u32) -> (String, bool) {
        let key_rune = rune_of(key_val);
        let is_printable_key = self.is_printable_key(state, key_val);
        let old_text = self.get_preedit_string();

        let key_s = if is_printable_key {
            key_rune.to_string()
        } else {
            String::new()
        };

        if is_printable_key && self.preeditor.can_process_key(key_rune) {
            let mut key_rune = key_rune;
            if state & FCITX_LOCK_MASK != 0 {
                key_rune = self.to_upper(key_rune);
            }
            let mode = self.get_input_mode();
            self.preeditor.process_key(key_rune, mode);

            if self
                .preeditor
                .input_method()
                .appending_keys
                .contains(&key_rune)
            {
                let new_text = if self.should_fallback_to_english(true) {
                    self.get_processed_string(Mode::ENGLISH)
                } else {
                    self.get_processed_string(Mode::VIETNAMESE)
                };
                let full_seq = self.get_processed_string(Mode::VIETNAMESE);
                if full_seq.ends_with(key_rune) {
                    // [[ => [
                    let ret = self.get_preedit_string();
                    let is_word_break = ret.chars().last().is_some_and(is_word_break_symbol);
                    if is_word_break {
                        self.preeditor.remove_last_char(false);
                        self.preeditor.process_key(' ', Mode::ENGLISH);
                    }
                    return (ret, is_word_break);
                } else if new_text.ends_with(key_rune) {
                    // f] => f]
                    let is_word_break = is_word_break_symbol(key_rune);
                    if is_word_break {
                        self.preeditor.remove_last_char(false);
                        self.preeditor.process_key(' ', Mode::ENGLISH);
                    }
                    return (format!("{old_text}{key_rune}"), is_word_break);
                } else {
                    // ] => o?
                    return (self.get_preedit_string(), false);
                }
            } else if self.options.macro_enabled {
                return (self.get_processed_string(Mode::PUNCTUATION), false);
            } else {
                return (self.get_preedit_string(), false);
            }
        } else if self.options.macro_enabled {
            if is_printable_key && self.macro_table.has_prefix(&format!("{old_text}{key_s}")) {
                self.preeditor.process_key(key_rune, Mode::ENGLISH);
                return (format!("{old_text}{key_s}"), false);
            }
            if self.macro_table.has_key(&old_text) {
                if is_printable_key {
                    return (format!("{}{key_s}", self.expand_macro(&old_text)), true);
                }
                return (self.expand_macro(&old_text), true);
            }
        }
        (self.handle_non_vn_word(key_val, state), true)
    }

    fn handle_non_vn_word(&mut self, key_val: u32, state: u32) -> String {
        let key_rune = rune_of(key_val);
        let is_printable_key = self.is_printable_key(state, key_val);
        let old_text = self.get_preedit_string();
        let key_s = if is_printable_key {
            key_rune.to_string()
        } else {
            String::new()
        };

        if has_any_vietnamese_rune(&old_text) && self.must_fallback_to_english() {
            self.preeditor.restore_last_word(false);
            let new_text = format!(
                "{}{key_s}",
                self.preeditor
                    .get_processed_string(Mode::PUNCTUATION | Mode::ENGLISH)
            );
            if is_printable_key {
                self.preeditor.process_key(key_rune, Mode::ENGLISH);
            }
            return new_text;
        }
        if is_printable_key {
            self.preeditor.process_key(key_rune, Mode::ENGLISH);
            return format!("{old_text}{key_s}");
        }
        // Ctrl + A is treated as a word break
        format!("{old_text}{key_s}")
    }

    // =====================================================================
    // Fallback policy
    // =====================================================================

    fn get_input_mode(&self) -> Mode {
        if self.should_fallback_to_english(false) {
            Mode::ENGLISH
        } else {
            Mode::VIETNAMESE
        }
    }

    fn should_fallback_to_english(&self, check_vn_rune: bool) -> bool {
        if !self.options.auto_non_vn_restore {
            return false;
        }
        let vn_seq = self.get_processed_string(Mode::VIETNAMESE | Mode::LOWER_CASE);
        let vn_runes: Vec<char> = vn_seq.chars().collect();
        if vn_runes.is_empty() {
            return false;
        }
        if self.get_macro_text().0 {
            return false;
        }
        // `dd` is allowed in non-Vietnamese sequences, it is used a lot in abbreviations.
        if self.options.dd_free_style
            && !has_any_vietnamese_vowel(&vn_seq)
            && (vn_runes.last() == Some(&'d') || vn_seq.contains('đ'))
        {
            return false;
        }
        if check_vn_rune && !has_any_vietnamese_rune(&vn_seq) {
            return false;
        }
        !self.preeditor.is_valid(false)
    }

    fn must_fallback_to_english(&self) -> bool {
        if !self.options.auto_non_vn_restore {
            return false;
        }
        let vn_seq = self.get_processed_string(Mode::VIETNAMESE | Mode::LOWER_CASE);
        if vn_seq.is_empty() {
            return false;
        }
        if self.options.dd_free_style && vn_seq.contains('đ') {
            return false;
        }
        if self.options.spell_check_with_dicts {
            return !self.dictionary.contains(&vn_seq);
        }
        !self.preeditor.is_valid(true)
    }

    fn get_composed_string(&self, old_text: &str) -> String {
        if has_any_vietnamese_rune(old_text) && self.must_fallback_to_english() {
            return self.get_processed_string(Mode::ENGLISH);
        }
        old_text.to_string()
    }

    // =====================================================================
    // Macros
    // =====================================================================

    fn get_macro_text(&self) -> (bool, String) {
        if !self.options.macro_enabled {
            return (false, String::new());
        }
        let text = self.get_processed_string(Mode::PUNCTUATION);
        if self.macro_table.has_key(&text) {
            return (true, self.expand_macro(&text));
        }
        (false, String::new())
    }

    fn expand_macro(&self, text: &str) -> String {
        let macro_text = self.macro_table.get_text(text);
        if self.options.auto_capitalize_macro {
            match determine_macro_case(text) {
                MacroCase::AllSmall => return macro_text.to_lowercase(),
                MacroCase::AllCapital => return macro_text.to_uppercase(),
                MacroCase::NoChange => {}
            }
        }
        macro_text
    }

    // =====================================================================
    // Buffers and small helpers
    // =====================================================================

    fn get_preedit_string(&self) -> String {
        if self.options.macro_enabled {
            return self.get_processed_string(Mode::PUNCTUATION);
        }
        if self.should_fallback_to_english(true) {
            return self.get_processed_string(Mode::ENGLISH);
        }
        self.get_processed_string(Mode::VIETNAMESE)
    }

    fn get_processed_string(&self, mode: Mode) -> String {
        self.preeditor.get_processed_string(mode)
    }

    fn encode_text(&self, text: &str) -> String {
        encode(&self.options.output_charset, text)
    }

    fn update_preedit(&mut self, processed_str: &str) {
        // The preedit stays readable Unicode; the charset applies on commit.
        if processed_str.is_empty() {
            // Upstream clears both buffers here, including the commit string.
            self.preedit_text.clear();
            self.commit_text.clear();
            return;
        }
        self.preedit_text = processed_str.to_string();
    }

    /// The single place text leaves the engine: every commit (word break,
    /// macro, restore, mode flush, reset) is encoded here, exactly once.
    fn commit_preedit_and_reset(&mut self, text: &str) {
        self.commit_text = self.encode_text(text);
        self.preedit_text.clear();
        self.preeditor.reset();
    }

    fn get_raw_key_len(&self) -> usize {
        self.preeditor
            .get_processed_string(Mode::ENGLISH | Mode::FULL_TEXT)
            .chars()
            .count()
    }

    fn rune_count(&self) -> usize {
        self.get_preedit_string().chars().count()
    }

    /// `[`/`]`/`{`/`}` act as mark keys in some layouts, with swapped case pairs.
    fn to_upper(&self, key_rune: char) -> char {
        let mapped = match key_rune {
            '[' => Some('{'),
            ']' => Some('}'),
            '{' => Some('['),
            '}' => Some(']'),
            _ => None,
        };
        match mapped {
            Some(upper)
                if self
                    .preeditor
                    .input_method()
                    .appending_keys
                    .contains(&key_rune) =>
            {
                upper
            }
            _ => key_rune,
        }
    }

    fn update_last_key_with_shift(&mut self, key_val: u32, state: u32) {
        let key_rune = rune_of(key_val);
        self.last_key_with_shift =
            self.preeditor.can_process_key(key_rune) && state & FCITX_SHIFT_MASK != 0;
    }

    fn is_printable_key(&self, state: u32, key_val: u32) -> bool {
        is_valid_state(state) && self.is_valid_key_val(key_val)
    }

    fn is_valid_key_val(&self, key_val: u32) -> bool {
        let key_rune = rune_of(key_val);
        if key_val == FCITX_BACKSPACE || is_word_break_symbol(key_rune) {
            return true;
        }
        if self.get_macro_text().0 && key_val == FCITX_TAB {
            return true;
        }
        self.preeditor.can_process_key(key_rune)
    }
}

fn rune_of(key_val: u32) -> char {
    char::from_u32(key_val).unwrap_or('\u{fffd}')
}

pub fn is_valid_state(state: u32) -> bool {
    state & FCITX_CONTROL_MASK == 0
        && state & FCITX_MOD1_MASK == 0
        && state & FCITX_IGNORED_MASK == 0
        && state & FCITX_SUPER_MASK == 0
        && state & FCITX_HYPER_MASK == 0
        && state & FCITX_META_MASK == 0
}
