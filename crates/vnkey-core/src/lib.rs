//! Vietnamese input method engine.
//!
//! Composition model, rule DSL and phonotactic tables for Telex/VNI/VIQR-style
//! input. The data tables in `charset` and `im_defs` are hand-maintained: treat
//! a change there as a behaviour change and cover it with a test.
//!
//! See `NOTICE` for the provenance of the ported portions.

mod charset;
mod im_defs;

pub mod encoder;
pub mod engine;
pub mod flattener;
pub mod mode;
pub mod rules;
pub mod rules_parser;
pub mod spelling;
pub mod tables;
pub mod utils;

pub use encoder::{charset_names, encode};
pub use engine::Engine;
pub use mode::{Flags, Mode};
pub use rules::{EffectType, InputMethod, Mark, Rule, Tone};
pub use rules_parser::{input_method_names, parse_input_method, parse_input_methods};
