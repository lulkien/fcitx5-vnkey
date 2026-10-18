//! Input-method definition DSL.
//!
//! Upstream parses each definition line with two regular expressions. They are
//! hand-parsed here to keep the crate dependency-free; the semantics (leftmost
//! match, greedy runs) are preserved and pinned by the ported parser tests.

use crate::im_defs::INPUT_METHOD_DEFINITIONS;
use crate::rules::{EffectType, InputMethod, Mark, Rule, Tone};
use crate::utils::{add_tone_to_char, mark_family, find_mark_from_char, is_vowel};

fn tone_from_name(name: &str) -> Option<Tone> {
    match name {
        "XoaDauThanh" => Some(Tone::None),
        "DauSac" => Some(Tone::Acute),
        "DauHuyen" => Some(Tone::Grave),
        "DauNga" => Some(Tone::Tilde),
        "DauNang" => Some(Tone::Dot),
        "DauHoi" => Some(Tone::Hook),
        _ => None,
    }
}

struct DslMatch {
    effective_on: String,
    results: String,
    tail: String,
}

/// `([a-zA-Z]+)_(\p{L}+)([_\p{L}]*)` — leftmost match.
fn find_dsl(line: &str) -> Option<DslMatch> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let run_start = i;
        while i < chars.len() && chars[i].is_ascii_alphabetic() {
            i += 1;
        }
        if i >= chars.len() || chars[i] != '_' {
            i = run_start + 1;
            continue;
        }
        let letters_start = i + 1;
        let mut j = letters_start;
        while j < chars.len() && chars[j].is_alphabetic() {
            j += 1;
        }
        if j == letters_start {
            i = run_start + 1;
            continue;
        }
        let tail_start = j;
        while j < chars.len() && (chars[j] == '_' || chars[j].is_alphabetic()) {
            j += 1;
        }
        return Some(DslMatch {
            effective_on: chars[run_start..i].iter().collect(),
            results: chars[letters_start..tail_start].iter().collect(),
            tail: chars[tail_start..j].iter().collect(),
        });
    }
    None
}

/// `(_?)_(\p{L}+)` — leftmost match, returns the captured letters.
///
/// The optional underscore is mutually exclusive with the mandatory one here:
/// when it is consumed the following character is a letter, so only the
/// non-greedy branch can produce a match.
fn find_appending(value: &str) -> Option<String> {
    let chars: Vec<char> = value.chars().collect();
    for p in 0..chars.len() {
        if chars[p] != '_' {
            continue;
        }
        for start in [p + 1, p + 2] {
            // p + 1 = optional underscore not taken, p + 2 = taken
            if start == p + 2 && chars.get(p + 1) != Some(&'_') {
                continue;
            }
            let mut j = start;
            while j < chars.len() && chars[j].is_alphabetic() {
                j += 1;
            }
            if j > start {
                return Some(chars[start..j].iter().collect());
            }
        }
    }
    None
}

pub fn parse_rules(key: char, line: &str) -> Vec<Rule> {
    match tone_from_name(line) {
        Some(tone) => vec![Rule {
            key,
            effect_type: Some(EffectType::Tone),
            effect: tone as u8,
            ..Rule::default()
        }],
        None => parse_toneless_rules(key, line),
    }
}

pub fn parse_toneless_rules(key: char, line: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    if find_dsl(line).is_some() {
        // Upstream matches on the original line, then extracts the groups from
        // the lowercased copy.
        let lowered = line.to_lowercase();
        let Some(matched) = find_dsl(&lowered) else {
            return rules;
        };
        let effective_ons: Vec<char> = matched.effective_on.chars().collect();
        let results: Vec<char> = matched.results.chars().collect();
        for (i, &effective_on) in effective_ons.iter().enumerate() {
            let Some(&result) = results.get(i) else { break };
            let Some(effect) = find_mark_from_char(result) else {
                continue;
            };
            rules.extend(parse_tone_less_rule(key, effective_on, result, effect));
        }
        if let Some(rule) = get_appending_rule(key, &matched.tail) {
            rules.push(rule);
        }
    } else if let Some(rule) = get_appending_rule(key, line) {
        rules.push(rule);
    }
    rules
}

fn parse_tone_less_rule(key: char, effective_on: char, result: char, effect: Mark) -> Vec<Rule> {
    let mut rules = Vec::new();
    for chr in mark_family(effective_on) {
        if chr == result {
            rules.push(Rule {
                key,
                effect_type: Some(EffectType::Mark),
                effect: Mark::None as u8,
                effect_on: result,
                result: effective_on,
                appended_rules: Vec::new(),
            });
        } else if is_vowel(chr) {
            for tone in 0u8..6 {
                rules.push(Rule {
                    key,
                    effect_type: Some(EffectType::Mark),
                    effect_on: add_tone_to_char(chr, tone),
                    effect: effect as u8,
                    result: add_tone_to_char(result, tone),
                    appended_rules: Vec::new(),
                });
            }
        } else {
            rules.push(Rule {
                key,
                effect_type: Some(EffectType::Mark),
                effect_on: chr,
                effect: effect as u8,
                result,
                appended_rules: Vec::new(),
            });
        }
    }
    rules
}

fn get_appending_rule(key: char, value: &str) -> Option<Rule> {
    let letters = find_appending(value)?;
    let chars: Vec<char> = letters.chars().collect();
    let first = *chars.first()?;
    let mut rule = Rule {
        key,
        effect_type: Some(EffectType::Appending),
        effect_on: first,
        result: first,
        ..Rule::default()
    };
    for &chr in chars.iter().skip(1) {
        rule.appended_rules.push(Rule {
            key,
            effect_type: Some(EffectType::Appending),
            effect_on: chr,
            result: chr,
            ..Rule::default()
        });
    }
    Some(rule)
}

/// Parse every definition in the generated table, in upstream file order.
pub fn parse_input_methods() -> Vec<InputMethod> {
    INPUT_METHOD_DEFINITIONS
        .iter()
        .map(|(name, definitions)| {
            let mut im = InputMethod {
                name: (*name).to_string(),
                ..InputMethod::default()
            };
            for (key_str, line) in definitions.iter() {
                let Some(key) = key_str.chars().next() else {
                    continue;
                };
                im.rules.extend(parse_rules(key, line));
                if line.to_lowercase().contains("uo") {
                    im.super_keys.push(key);
                }
                im.keys.push(key);
            }
            let mut appending_keys = Vec::new();
            let mut tone_keys = Vec::new();
            for rule in &im.rules {
                if rule.is_appending() {
                    appending_keys.push(rule.key);
                }
                if rule.is_tone() {
                    tone_keys.push(rule.key);
                }
            }
            im.appending_keys = appending_keys;
            im.tone_keys = tone_keys;
            im
        })
        .collect()
}

/// Parse one input method by name; `None` if the name is unknown.
pub fn parse_input_method(name: &str) -> Option<InputMethod> {
    parse_input_methods().into_iter().find(|im| im.name == name)
}

pub fn input_method_names() -> Vec<&'static str> {
    INPUT_METHOD_DEFINITIONS.iter().map(|(name, _)| *name).collect()
}
