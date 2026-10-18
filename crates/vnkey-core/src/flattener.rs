//! Composition rendering.


// Boolean structure mirrors the upstream Go expressions line for line;
// simplifying it would break the 1:1 diff against `vnkey-core`.
#![allow(clippy::nonminimal_bool, clippy::collapsible_if, clippy::if_same_then_else)]
use crate::engine::{Node, Transformation};
use crate::mode::Mode;
use crate::rules::{EffectType, Mark};
use crate::tables::{simple_lower, simple_upper};
use crate::utils::{add_mark_to_char, add_tone_to_char};

pub fn flatten(arena: &[Transformation], comp: &[Node], mode: Mode) -> String {
    get_canvas(arena, comp, mode).into_iter().collect()
}

/// Render the composition to characters. Nodes with no target and no effect are
/// dropped; appending nodes are rendered and their effect children folded in.
pub fn get_canvas(arena: &[Transformation], comp: &[Node], mode: Mode) -> Vec<char> {
    use std::collections::HashMap;

    let mut appending_map: HashMap<Node, Vec<Node>> = HashMap::new();
    let mut appending_list: Vec<Node> = Vec::new();

    for &idx in comp {
        let trans = &arena[idx];
        if mode.contains(Mode::ENGLISH) {
            if trans.rule.is_virtual() {
                continue;
            }
            appending_list.push(idx);
        } else if trans.rule.is_appending() {
            if trans.rule.is_virtual() {
                continue;
            }
            appending_list.push(idx);
        } else if let Some(target) = trans.target {
            appending_map.entry(target).or_default().push(idx);
        }
    }

    let mut canvas = Vec::with_capacity(appending_list.len());
    for appending_idx in appending_list {
        let appending = &arena[appending_idx];
        let mut chr;
        if mode.contains(Mode::ENGLISH) {
            chr = appending.rule.key;
        } else {
            chr = appending.rule.effect_on;
            if let Some(children) = appending_map.get(&appending_idx) {
                for &child_idx in children {
                    let child = &arena[child_idx];
                    match child.rule.effect_type {
                        Some(EffectType::Mark) => {
                            if child.rule.effect == Mark::Raw as u8 {
                                chr = appending.rule.key;
                            } else {
                                chr = add_mark_to_char(chr, child.rule.effect);
                            }
                        }
                        Some(EffectType::Tone) => {
                            chr = add_tone_to_char(chr, child.rule.effect);
                        }
                        _ => {}
                    }
                }
            }
        }
        if mode.contains(Mode::TONE_LESS) {
            chr = add_tone_to_char(chr, 0);
        }
        if mode.contains(Mode::MARK_LESS) {
            chr = add_mark_to_char(chr, 0);
        }
        if mode.contains(Mode::LOWER_CASE) {
            chr = simple_lower(chr);
        } else if appending.is_upper_case {
            chr = simple_upper(chr);
        }
        canvas.push(chr);
    }
    canvas
}
