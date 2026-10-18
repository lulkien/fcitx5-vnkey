//! cxx bridge consumed by the fcitx5 façade (`cpp/facade.cpp`).
//!
//! Everything policy-related stays on this side; the façade only translates
//! fcitx5 key events and preedit/commit calls.

use std::collections::HashSet;

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
        fn set_restore_key_stroke(self: &mut EngineHandle);
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
/// the façade then passes every key through instead of failing to load.
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

    pub fn set_restore_key_stroke(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.set_restore_key_stroke();
        }
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

pub fn create(
    input_method: &str,
    macros: Vec<ffi::MacroPair>,
    dictionary_path: &str,
) -> Box<EngineHandle> {
    let dictionary: HashSet<String> = if dictionary_path.is_empty() {
        dictionary::vietnamese()
    } else {
        match std::fs::read_to_string(dictionary_path) {
            Ok(text) => dictionary::from_lines(&text),
            // An unreadable override falls back to the embedded dictionary
            // rather than silently disabling spell checking.
            Err(_) => dictionary::vietnamese(),
        }
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
