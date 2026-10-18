//! Engine options.

#[derive(Clone, Debug)]
pub struct Options {
    /// Restore raw keystrokes when the word is not a valid Vietnamese one.
    pub auto_non_vn_restore: bool,
    /// Allow `dd` even in otherwise non-Vietnamese sequences (abbreviations).
    pub dd_free_style: bool,
    pub macro_enabled: bool,
    pub auto_capitalize_macro: bool,
    /// Validate with the dictionary instead of the phonotactic tables.
    pub spell_check_with_dicts: bool,
    pub output_charset: String,
    /// Modern tone placement (`òa`), i.e. upstream `!std_tone_style`.
    pub modern_style: bool,
    /// Allow typing marks/tone in any position.
    pub free_marking: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            auto_non_vn_restore: true,
            dd_free_style: true,
            macro_enabled: false,
            auto_capitalize_macro: false,
            spell_check_with_dicts: true,
            output_charset: "Unicode".to_string(),
            modern_style: false,
            free_marking: true,
        }
    }
}

impl Options {
    /// Translate the addon options into engine flags, as `EngineSetOption` does.
    pub fn flags(&self) -> vnkey_core::Flags {
        let mut flags = vnkey_core::Flags::STD;
        flags.set(vnkey_core::Flags::STD_TONE_STYLE, !self.modern_style);
        flags.set(vnkey_core::Flags::FREE_TONE_MARKING, self.free_marking);
        flags
    }
}
