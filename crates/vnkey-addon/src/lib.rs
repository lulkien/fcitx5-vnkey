//! The addon-side API: preedit/commit state machine on top of `vnkey-core`.
//!
//! LGPL-2.1-or-later; see NOTICE for the origin of the ported portions.

#[cfg(feature = "ffi")]
pub mod bridge;
pub mod dictionary;
pub mod engine;
pub mod macro_table;
pub mod options;

pub use engine::{
    determine_macro_case, is_valid_state, VnKeyEngine, MacroCase, FCITX_BACKSPACE,
    FCITX_CONTROL_MASK, FCITX_IGNORED_MASK, FCITX_LOCK_MASK, FCITX_MOD1_MASK, FCITX_SHIFT_MASK,
    FCITX_SPACE, FCITX_TAB,
};
pub use macro_table::MacroTable;
pub use options::Options;
