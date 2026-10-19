//! cxx bridge consumed by the fcitx5 facade (`cpp/facade.cpp`).
//!
//! Everything policy-related stays on this side; the facade only translates
//! fcitx5 key events and preedit/commit calls.

use std::collections::HashSet;
use std::io::Read;
use std::sync::Arc;

use crate::dictionary;
use crate::engine::VnKeyEngine;
use crate::macro_table::MacroTable;
use crate::options::Options;

#[cxx::bridge(namespace = "vnkey")]
mod ffi {
    /// One macro entry (key, replacement).
    pub struct MacroPair {
        pub key: String,
        pub value: String,
    }

    /// Addon options, one-to-one with the fcitx5 configuration options.
    pub struct EngineOptions {
        pub auto_non_vn_restore: bool,
        pub macro_enabled: bool,
        pub auto_capitalize_macro: bool,
        pub spell_check_with_dicts: bool,
        pub modern_style: bool,
        pub free_marking: bool,
        pub output_charset: String,
    }

    extern "Rust" {
        type EngineHandle;

        /// `valid()` is false when the input method name is unknown.
        fn create(
            input_method: &str,
            macros: Vec<MacroPair>,
            dictionary_path: &str,
        ) -> Box<EngineHandle>;

        fn valid(self: &EngineHandle) -> bool;
        fn process_key_event(self: &mut EngineHandle, key_val: u32, state: u32) -> bool;
        fn set_options(self: &mut EngineHandle, options: EngineOptions);
        fn restore_key_stroke(self: &mut EngineHandle) -> bool;
        fn set_vietnamese_mode(self: &mut EngineHandle, enabled: bool);
        fn vietnamese_mode(self: &EngineHandle) -> bool;
        fn pull_preedit(self: &EngineHandle) -> String;
        fn pull_commit(self: &mut EngineHandle) -> String;
        fn commit_preedit(self: &mut EngineHandle);
        fn reset(self: &mut EngineHandle);
    }

    extern "Rust" {
        fn charset_names() -> Vec<String>;
        fn input_method_names() -> Vec<String>;
    }
}

/// One engine per fcitx5 `InputContext`.
///
/// `engine` is `None` only when the configured input method name is unknown;
/// the facade then passes every key through instead of failing to load.
pub struct EngineHandle {
    engine: Option<VnKeyEngine>,
}

impl EngineHandle {
    pub fn valid(&self) -> bool {
        self.engine.is_some()
    }

    pub fn process_key_event(&mut self, key_val: u32, state: u32) -> bool {
        match self.engine.as_mut() {
            Some(engine) => engine.process_key_event(key_val, state),
            None => false,
        }
    }

    pub fn set_options(&mut self, options: ffi::EngineOptions) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };
        engine.set_options(Options {
            auto_non_vn_restore: options.auto_non_vn_restore,
            dd_free_style: true,
            macro_enabled: options.macro_enabled,
            auto_capitalize_macro: options.auto_capitalize_macro,
            spell_check_with_dicts: options.spell_check_with_dicts,
            output_charset: options.output_charset,
            modern_style: options.modern_style,
            free_marking: options.free_marking,
        });
    }

    pub fn restore_key_stroke(&mut self) -> bool {
        self.engine
            .as_mut()
            .is_some_and(VnKeyEngine::restore_key_stroke)
    }

    pub fn set_vietnamese_mode(&mut self, enabled: bool) {
        if let Some(engine) = self.engine.as_mut() {
            engine.set_vietnamese_mode(enabled);
        }
    }

    pub fn vietnamese_mode(&self) -> bool {
        self.engine
            .as_ref()
            .map(VnKeyEngine::vietnamese_mode)
            .unwrap_or(true)
    }

    pub fn pull_preedit(&self) -> String {
        self.engine
            .as_ref()
            .map(|engine| engine.pull_preedit().to_string())
            .unwrap_or_default()
    }

    pub fn pull_commit(&mut self) -> String {
        self.engine
            .as_mut()
            .map(VnKeyEngine::pull_commit)
            .unwrap_or_default()
    }

    pub fn commit_preedit(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.commit_preedit();
        }
    }

    pub fn reset(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.reset();
        }
    }
}

/// Refuse absurd dictionary overrides instead of reading them into memory.
const MAX_DICTIONARY_BYTES: u64 = 16 << 20;

/// Read an override through one handle: the type check and the size cap apply
/// to the file actually read, and `take` bounds the read even when the file
/// reports `st_size == 0` (procfs, some FUSE).
fn load_dictionary_override(path: &str) -> Option<Arc<HashSet<String>>> {
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    file.take(MAX_DICTIONARY_BYTES + 1)
        .read_to_string(&mut text)
        .ok()?;
    if text.len() as u64 > MAX_DICTIONARY_BYTES {
        return None;
    }
    Some(Arc::new(dictionary::from_lines(&text)))
}

pub fn create(
    input_method: &str,
    macros: Vec<ffi::MacroPair>,
    dictionary_path: &str,
) -> Box<EngineHandle> {
    // An empty path means the embedded dictionary; an override that is
    // unreadable or unreasonably large falls back to it rather than silently
    // disabling spell checking.
    let dictionary = if dictionary_path.is_empty() {
        dictionary::vietnamese()
    } else {
        load_dictionary_override(dictionary_path).unwrap_or_else(dictionary::vietnamese)
    };

    let table = MacroTable::from_pairs(macros.into_iter().map(|pair| (pair.key, pair.value)));
    Box::new(EngineHandle {
        engine: VnKeyEngine::new(input_method, dictionary, table, Options::default()),
    })
}

pub fn charset_names() -> Vec<String> {
    vnkey_core::charset_names()
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub fn input_method_names() -> Vec<String> {
    vnkey_core::input_method_names()
        .into_iter()
        .map(str::to_string)
        .collect()
}
