//! Composition engine.
//!
//! Upstream stores the composition as a linked list of heap nodes
//! (`[]*Transformation` with `Target` back-pointers). Here the nodes live in an
//! append-only arena and are referenced by index — that is what makes the port
//! safe Rust without any reference counting, at the cost of transient probe
//! nodes accumulating until the next `reset()`. Probe allocations are a handful
//! per keystroke and the arena is cleared on reset (i.e. on every commit /
//! focus change), so the steady-state footprint is bounded by one word.


// Boolean structure mirrors the upstream Go expressions line for line;
// simplifying it would break the 1:1 diff against `vnkey-core`.
#![allow(clippy::nonminimal_bool, clippy::collapsible_if, clippy::if_same_then_else)]
use crate::mode::{Flags, Mode};
use crate::rules::{EffectType, InputMethod, Mark, Rule, Tone};
use crate::spelling::is_valid_cvc;
use crate::tables::simple_lower;
use crate::utils::{can_process_key, find_tone_from_char, in_key_list, is_alpha, is_space, is_vowel};

pub type Node = usize;

#[derive(Clone, Debug)]
pub struct Transformation {
    pub rule: Rule,
    pub target: Option<Node>,
    pub is_upper_case: bool,
}

impl Transformation {
    fn appending(key: char, is_upper_case: bool) -> Transformation {
        Transformation {
            rule: Rule {
                key,
                effect_on: key,
                effect_type: Some(EffectType::Appending),
                result: key,
                effect: 0,
                appended_rules: Vec::new(),
            },
            target: None,
            is_upper_case,
        }
    }

    fn probe(target: Option<Node>, rule: Rule) -> Transformation {
        Transformation {
            rule,
            target,
            is_upper_case: false,
        }
    }
}

fn alloc(arena: &mut Vec<Transformation>, trans: Transformation) -> Node {
    arena.push(trans);
    arena.len() - 1
}

// =========================================================================
// Lookup helpers
// =========================================================================

fn find_last_appending_trans(arena: &[Transformation], comp: &[Node]) -> Option<Node> {
    comp.iter()
        .rev()
        .copied()
        .find(|&idx| arena[idx].rule.is_appending())
}

fn filter_appending_composition(arena: &[Transformation], comp: &[Node]) -> Vec<Node> {
    comp.iter()
        .copied()
        .filter(|&idx| arena[idx].rule.is_appending())
        .collect()
}

fn find_root_target(arena: &[Transformation], mut target: Node) -> Node {
    while let Some(next) = arena[target].target {
        target = next;
    }
    target
}

fn get_last_tone_transformation(arena: &[Transformation], comp: &[Node]) -> Option<Node> {
    comp.iter().rev().copied().find(|&idx| {
        let trans = &arena[idx];
        trans.rule.is_tone() && trans.target.is_some()
    })
}

fn is_free(arena: &[Transformation], comp: &[Node], target: Option<Node>, effect: EffectType) -> bool {
    !comp.iter().any(|&idx| {
        let trans = &arena[idx];
        trans.target == target && trans.rule.effect_type == Some(effect)
    })
}

// =========================================================================
// Syllable structure extraction
// =========================================================================

/// Walk back over the nodes that belong to the same character class.
fn extract_atomic_trans(
    arena: &[Transformation],
    comp: &[Node],
    mut last: Vec<Node>,
    last_is_vowel: bool,
) -> (Vec<Node>, Vec<Node>) {
    let mut rest: Vec<Node> = comp.to_vec();
    while let Some(&idx) = rest.last() {
        let trans = &arena[idx];
        if trans.target.is_none() && is_vowel(trans.rule.result) != last_is_vowel {
            break;
        }
        rest.pop();
        last.insert(0, idx);
    }
    (rest, last)
}

fn extract_cvc_appending_trans(
    arena: &[Transformation],
    comp: &[Node],
) -> (Vec<Node>, Vec<Node>, Vec<Node>) {
    let (head, last_consonant) = extract_atomic_trans(arena, comp, Vec::new(), false);
    let (mut first_consonant, mut vowel) = extract_atomic_trans(arena, &head, Vec::new(), true);

    let mut last_consonant = last_consonant;
    if !last_consonant.is_empty() && vowel.is_empty() && first_consonant.is_empty() {
        first_consonant = last_consonant;
        vowel = Vec::new();
        last_consonant = Vec::new();
    }

    // 'gi' and 'qu' are qualified consonants: ['g','ia',''] -> ['gi','a','']
    if first_consonant.len() == 1
        && !vowel.is_empty()
        && ((arena[first_consonant[0]].rule.result == 'g'
            && arena[vowel[0]].rule.result == 'i'
            && vowel.len() > 1
            && !(arena[vowel[1]].rule.result == 'e' && !last_consonant.is_empty()))
            || (arena[first_consonant[0]].rule.result == 'q'
                && arena[vowel[0]].rule.result == 'u'))
    {
        first_consonant.push(vowel[0]);
        vowel.remove(0);
    }
    (first_consonant, vowel, last_consonant)
}

fn extract_cvc_trans(arena: &[Transformation], comp: &[Node]) -> (Vec<Node>, Vec<Node>, Vec<Node>) {
    let mut children: std::collections::HashMap<Node, Vec<Node>> = std::collections::HashMap::new();
    let mut appending_list: Vec<Node> = Vec::new();
    for &idx in comp {
        let trans = &arena[idx];
        match trans.target {
            None => appending_list.push(idx),
            Some(target) => children.entry(target).or_default().push(idx),
        }
    }

    let (mut fc, mut vo, mut lc) = extract_cvc_appending_trans(arena, &appending_list);
    // Upstream appends the children of the *original* members only: `range` over a
    // slice does not see elements appended during iteration.
    let base_fc = fc.clone();
    for idx in base_fc {
        if let Some(kids) = children.get(&idx) {
            fc.extend_from_slice(kids);
        }
    }
    let base_vo = vo.clone();
    for idx in base_vo {
        if let Some(kids) = children.get(&idx) {
            vo.extend_from_slice(kids);
        }
    }
    let base_lc = lc.clone();
    for idx in base_lc {
        if let Some(kids) = children.get(&idx) {
            lc.extend_from_slice(kids);
        }
    }
    (fc, vo, lc)
}

fn get_right_most_vowels(arena: &[Transformation], comp: &[Node]) -> Vec<Node> {
    extract_cvc_trans(arena, comp).1
}

/// Split the composition into `(everything before the last word, last word)`.
fn extract_last_word(
    arena: &[Transformation],
    comp: &[Node],
    effect_keys: &[char],
) -> (Vec<Node>, Vec<Node>) {
    if comp.is_empty() {
        return (Vec::new(), Vec::new());
    }
    for i in (0..comp.len()).rev() {
        let canvas = crate::flattener::get_canvas(
            arena,
            &comp[i..],
            Mode::VIETNAMESE | Mode::LOWER_CASE | Mode::TONE_LESS | Mode::MARK_LESS,
        );
        let Some(&c) = canvas.first() else { continue };
        if !is_alpha(c) && !in_key_list(effect_keys, c) {
            if i == comp.len() - 1 {
                return (comp.to_vec(), Vec::new());
            }
            return (comp[..=i].to_vec(), comp[i + 1..].to_vec());
        }
    }
    (Vec::new(), comp.to_vec())
}

fn extract_last_word_with_punctuation_marks(
    arena: &[Transformation],
    comp: &[Node],
    _effect_keys: &[char],
) -> (Vec<Node>, Vec<Node>) {
    if comp.is_empty() {
        return (Vec::new(), Vec::new());
    }
    for i in (0..comp.len()).rev() {
        let canvas = crate::flattener::get_canvas(arena, &comp[i..], Mode::ENGLISH);
        let Some(&c) = canvas.first() else { continue };
        if is_space(c) {
            if i == comp.len() - 1 {
                return (comp.to_vec(), Vec::new());
            }
            return (comp[..=i].to_vec(), comp[i + 1..].to_vec());
        }
    }
    (Vec::new(), comp.to_vec())
}

fn extract_last_syllable(arena: &[Transformation], comp: &[Node]) -> (Vec<Node>, Vec<Node>) {
    let (mut previous, last) = extract_last_word(arena, comp, &[]);
    let mut anchor = 0usize;
    for i in 0..last.len() {
        if !is_valid(arena, &last[anchor..=i], false) {
            anchor = i;
        }
    }
    if anchor > 0 {
        previous.extend_from_slice(&last[..anchor]);
    }
    (previous, last[anchor..].to_vec())
}

// =========================================================================
// Validity
// =========================================================================

fn is_valid(arena: &[Transformation], comp: &[Node], input_is_full_complete: bool) -> bool {
    if comp.len() <= 1 {
        return true;
    }
    for i in (0..comp.len()).rev() {
        let rule = &arena[comp[i]].rule;
        if rule.is_tone() {
            if !has_valid_tone(arena, comp, rule.tone()) {
                return false;
            }
            break;
        }
    }
    let (fc, vo, lc) = extract_cvc_trans(arena, comp);
    let mode = Mode::VIETNAMESE | Mode::LOWER_CASE | Mode::TONE_LESS;
    is_valid_cvc(
        &crate::flattener::flatten(arena, &fc, mode),
        &crate::flattener::flatten(arena, &vo, mode),
        &crate::flattener::flatten(arena, &lc, mode),
        input_is_full_complete,
    )
}

// =========================================================================
// Tone targeting
// =========================================================================

/// A stop coda (`c k p t ch`) only accepts the acute or dot tone.
fn has_valid_tone(arena: &[Transformation], comp: &[Node], tone: Tone) -> bool {
    if tone == Tone::None || tone == Tone::Acute || tone == Tone::Dot {
        return true;
    }
    let (_, _, lc) = extract_cvc_trans(arena, comp);
    if lc.is_empty() {
        return true;
    }
    let last_consonants =
        crate::flattener::flatten(arena, &lc, Mode::ENGLISH | Mode::LOWER_CASE);
    !matches!(last_consonants.as_str(), "c" | "k" | "p" | "t" | "ch")
}

fn find_tone_target(
    arena: &[Transformation],
    comp: &[Node],
    std_style: bool,
) -> Option<Node> {
    if comp.is_empty() {
        return None;
    }
    let (_, vo, lc) = extract_cvc_trans(arena, comp);
    let vowels = filter_appending_composition(arena, &vo);
    let mut target = None;

    match vowels.len() {
        1 => target = Some(vowels[0]),
        2 if std_style => {
            for &idx in &vo {
                let result = arena[idx].rule.result;
                if result == 'ơ' || result == 'ê' {
                    target = Some(arena[idx].target.unwrap_or(idx));
                }
            }
            if target.is_none() {
                target = Some(if lc.is_empty() { vowels[0] } else { vowels[1] });
            }
        }
        2 => {
            if !lc.is_empty() {
                target = Some(vowels[1]);
            } else {
                let mode =
                    Mode::ENGLISH | Mode::LOWER_CASE | Mode::TONE_LESS | Mode::MARK_LESS;
                let flattened = crate::flattener::flatten(arena, &vowels, mode);
                target = Some(match flattened.as_str() {
                    "oa" | "oe" | "uy" | "ue" | "uo" => vowels[1],
                    _ => vowels[0],
                });
            }
        }
        3 => {
            let mode =
                Mode::ENGLISH | Mode::LOWER_CASE | Mode::TONE_LESS | Mode::MARK_LESS;
            let flattened = crate::flattener::flatten(arena, &vowels, mode);
            target = Some(if flattened == "uye" {
                vowels[2]
            } else {
                vowels[1]
            });
        }
        _ => {}
    }
    target
}

fn find_mark_target(
    arena: &mut Vec<Transformation>,
    comp: &[Node],
    rules: &[Rule],
) -> Option<(Node, Rule)> {
    let str_ = crate::flattener::flatten(arena, comp, Mode::VIETNAMESE);
    let mut i = comp.len();
    while i > 0 {
        i -= 1;
        let trans_rule_result = arena[comp[i]].rule.result;
        for rule in rules {
            if !rule.is_mark() || rule.effect == 0 {
                continue;
            }
            if trans_rule_result != rule.effect_on {
                continue;
            }
            let target = find_root_target(arena, comp[i]);
            let probe = alloc(arena, Transformation::probe(Some(target), rule.clone()));
            let mut tmp: Vec<Node> = comp.to_vec();
            tmp.push(probe);
            if str_ == crate::flattener::flatten(arena, &tmp, Mode::VIETNAMESE) {
                continue;
            }
            if is_valid(arena, &tmp, false) {
                return Some((target, rule.clone()));
            }
        }
    }
    None
}

fn find_target(
    arena: &mut Vec<Transformation>,
    comp: &[Node],
    applicable_rules: &[Rule],
    flags: Flags,
) -> Option<(Option<Node>, Rule)> {
    let str_ = crate::flattener::flatten(arena, comp, Mode::VIETNAMESE);
    for rule in applicable_rules {
        if !rule.is_tone() {
            continue;
        }
        let mut target: Option<Node> = None;
        if flags.contains(Flags::FREE_TONE_MARKING) {
            if has_valid_tone(arena, comp, rule.tone()) {
                target = find_tone_target(arena, comp, flags.contains(Flags::STD_TONE_STYLE));
            }
        } else if let Some(last) = find_last_appending_trans(arena, comp) {
            if is_vowel(arena[last].rule.effect_on) {
                target = Some(last);
            }
        }

        let probe = alloc(arena, Transformation::probe(target, rule.clone()));
        let mut tmp: Vec<Node> = comp.to_vec();
        tmp.push(probe);
        if str_ == crate::flattener::flatten(arena, &tmp, Mode::VIETNAMESE) {
            continue;
        }
        // Upstream dereferences `target` here without a nil check; the probe
        // comparison above always short-circuits for a nil target in practice.
        if rule.tone() == Tone::None
            && is_free(arena, comp, target, EffectType::Tone)
            && target.is_none_or(|t| find_tone_from_char(arena[t].rule.result) == Tone::None)
        {
            target = None;
        }
        return Some((target, rule.clone()));
    }
    find_mark_target(arena, comp, applicable_rules).map(|(target, rule)| (Some(target), rule))
}

// =========================================================================
// Transformation generation
// =========================================================================

fn generate_appending_trans(
    arena: &mut Vec<Transformation>,
    rules: &[Rule],
    lower_key: char,
    is_upper_case: bool,
) -> Node {
    for rule in rules {
        if rule.key == lower_key && rule.is_appending() {
            let mut rule = rule.clone();
            let upper = is_upper_case || rule.effect_on.is_uppercase();
            rule.effect_on = simple_lower(rule.effect_on);
            rule.result = rule.effect_on;
            return alloc(
                arena,
                Transformation {
                    rule,
                    target: None,
                    is_upper_case: upper,
                },
            );
        }
    }
    alloc(arena, Transformation::appending(lower_key, is_upper_case))
}

fn generate_fallback_transformations(
    arena: &mut Vec<Transformation>,
    rules: &[Rule],
    lower_key: char,
    is_upper_case: bool,
) -> Vec<Node> {
    let first = generate_appending_trans(arena, rules, lower_key, is_upper_case);
    let appended_rules = arena[first].rule.appended_rules.clone();
    let mut out = vec![first];
    for appended in appended_rules {
        let mut appended = appended;
        let upper = is_upper_case || appended.effect_on.is_uppercase();
        appended.key = '\0';
        appended.effect_on = simple_lower(appended.effect_on);
        appended.result = appended.effect_on;
        out.push(alloc(
            arena,
            Transformation {
                rule: appended,
                target: None,
                is_upper_case: upper,
            },
        ));
    }
    out
}

fn generate_undo_transformations(
    arena: &mut Vec<Transformation>,
    comp: &[Node],
    rules: &[Rule],
    flags: Flags,
) -> Vec<Node> {
    let mut out = Vec::new();
    let str_ = crate::flattener::flatten(
        arena,
        comp,
        Mode::VIETNAMESE | Mode::TONE_LESS | Mode::LOWER_CASE,
    );

    for rule in rules {
        if rule.is_tone() {
            let mut target: Option<Node> = None;
            if flags.contains(Flags::FREE_TONE_MARKING) {
                if has_valid_tone(arena, comp, rule.tone()) {
                    target = find_tone_target(arena, comp, flags.contains(Flags::STD_TONE_STYLE));
                }
            } else if let Some(last) = find_last_appending_trans(arena, comp) {
                if is_vowel(arena[last].rule.effect_on) {
                    target = Some(last);
                }
            }
            let Some(target) = target else { continue };
            out.push(alloc(
                arena,
                Transformation::probe(
                    Some(target),
                    Rule {
                        effect_type: Some(EffectType::Tone),
                        effect: 0,
                        key: '\0',
                        ..Rule::default()
                    },
                ),
            ));
        } else if rule.is_mark() {
            let mut i = comp.len();
            while i > 0 {
                i -= 1;
                if arena[comp[i]].rule.result != rule.effect_on {
                    continue;
                }
                let target = find_root_target(arena, comp[i]);
                let probe = alloc(
                    arena,
                    Transformation::probe(
                        Some(target),
                        Rule {
                            key: '\0',
                            effect_type: Some(EffectType::Mark),
                            effect: 0,
                            ..Rule::default()
                        },
                    ),
                );
                let mut tmp: Vec<Node> = comp.to_vec();
                tmp.push(probe);
                if str_ == crate::flattener::flatten(
                    arena,
                    &tmp,
                    Mode::VIETNAMESE | Mode::TONE_LESS | Mode::LOWER_CASE,
                ) {
                    continue;
                }
                out.push(probe);
            }
        }
    }
    out
}

/// `(uơ|ưo)\p{L}+` — unanchored.
fn matches_uoh_tail(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 2 < chars.len() {
        let head = (chars[i] == 'u' && chars[i + 1] == 'ơ')
            || (chars[i] == 'ư' && chars[i + 1] == 'o');
        if head && chars[i + 2].is_alphabetic() {
            return true;
        }
        i += 1;
    }
    false
}

fn generate_transformations(
    arena: &mut Vec<Transformation>,
    comp: &[Node],
    applicable_rules: &[Rule],
    flags: Flags,
    lower_key: char,
    is_upper_case: bool,
) -> Vec<Node> {
    let mut transformations: Vec<Node> = Vec::new();

    // Double typing an effect key undoes it and its effects, e.g. w + w -> w.
    if let Some(&last) = comp.last() {
        let rule = &arena[last].rule;
        if rule.is_appending() && rule.key == lower_key && rule.key != rule.result {
            return vec![alloc(
                arena,
                Transformation::probe(
                    Some(last),
                    Rule {
                        effect_type: Some(EffectType::Mark),
                        effect: Mark::Raw as u8,
                        key: '\0',
                        ..Rule::default()
                    },
                ),
            )];
        }
    }

    if let Some((target, applicable_rule)) = find_target(arena, comp, applicable_rules, flags) {
        if target.is_some() {
            transformations.push(alloc(
                arena,
                Transformation {
                    rule: applicable_rule.clone(),
                    target,
                    is_upper_case,
                },
            ));
            if !applicable_rule.is_mark() {
                return transformations;
            }
            let mut new_comp: Vec<Node> = comp.to_vec();
            new_comp.extend_from_slice(&transformations);
            if is_valid(arena, &new_comp, true) {
                return transformations;
            }
            // `uow` shortcut: a virtual horn rule targeting 'u' or 'o'.
            if let Some((Some(target), virtual_rule)) =
                find_target(arena, &new_comp, applicable_rules, flags)
            {
                let mut virtual_rule = virtual_rule;
                virtual_rule.key = '\0';
                transformations.push(alloc(
                    arena,
                    Transformation {
                        rule: virtual_rule,
                        target: Some(target),
                        is_upper_case: false,
                    },
                ));
            }
            return transformations;
        }
    }

    // ươ/ưo(i/c/ng) + o -> uô
    let flattened = crate::flattener::flatten(
        arena,
        comp,
        Mode::VIETNAMESE | Mode::TONE_LESS | Mode::LOWER_CASE,
    );
    if flattened.contains("ưo") || flattened.contains("ươ") {
        let vowels = filter_appending_composition(arena, &get_right_most_vowels(arena, comp));
        if let Some(&first_vowel) = vowels.first() {
            let probe = alloc(
                arena,
                Transformation::probe(
                    Some(first_vowel),
                    Rule {
                        effect_type: Some(EffectType::Mark),
                        key: '\0',
                        effect: Mark::None as u8,
                        ..Rule::default()
                    },
                ),
            );
            let mut tmp: Vec<Node> = comp.to_vec();
            tmp.push(probe);
            if let Some((Some(target), applicable_rule)) =
                find_target(arena, &tmp, applicable_rules, flags)
            {
                if target != first_vowel {
                    transformations.push(probe);
                    transformations.push(alloc(
                        arena,
                        Transformation {
                            rule: applicable_rule,
                            target: Some(target),
                            is_upper_case,
                        },
                    ));
                    return transformations;
                }
            }
        }
    }

    let undo = generate_undo_transformations(arena, comp, applicable_rules, flags);
    if !undo.is_empty() {
        transformations.extend(undo);
        transformations.push(alloc(
            arena,
            Transformation::appending(lower_key, is_upper_case),
        ));
    }
    transformations
}

fn break_composition(arena: &mut Vec<Transformation>, comp: &[Node]) -> Vec<Node> {
    let mut out = Vec::new();
    for &idx in comp {
        let trans = &arena[idx];
        if trans.rule.is_virtual() {
            continue;
        }
        let key = trans.rule.key;
        let upper = trans.is_upper_case;
        out.push(alloc(arena, Transformation::appending(key, upper)));
    }
    out
}

fn refresh_last_tone_target(
    arena: &mut Vec<Transformation>,
    comp: &[Node],
    std_style: bool,
) -> Vec<Node> {
    let mut out = Vec::new();
    let rightmost_vowels = get_right_most_vowels(arena, comp);
    let Some(last_tone) = get_last_tone_transformation(arena, comp) else {
        return out;
    };
    if rightmost_vowels.is_empty() {
        return out;
    }
    let new_target = find_tone_target(arena, comp, std_style);
    if arena[last_tone].target != new_target {
        arena[last_tone].target = new_target;
        out.push(alloc(
            arena,
            Transformation::probe(
                new_target,
                Rule {
                    key: '\0',
                    effect_type: Some(EffectType::Tone),
                    effect: Tone::None as u8,
                    ..Rule::default()
                },
            ),
        ));
        let mut override_rule = arena[last_tone].rule.clone();
        override_rule.key = '\0';
        out.push(alloc(
            arena,
            Transformation::probe(new_target, override_rule),
        ));
    }
    out
}

// =========================================================================
// Engine
// =========================================================================

pub struct Engine {
    input_method: InputMethod,
    flags: Flags,
    arena: Vec<Transformation>,
    composition: Vec<Node>,
}

impl Engine {
    pub fn new(input_method: InputMethod, flags: Flags) -> Engine {
        Engine {
            input_method,
            flags,
            arena: Vec::new(),
            composition: Vec::new(),
        }
    }

    pub fn input_method(&self) -> &InputMethod {
        &self.input_method
    }

    pub fn set_flags(&mut self, flags: Flags) {
        self.flags = flags;
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn composition(&self) -> &[Node] {
        &self.composition
    }

    pub fn is_valid(&self, input_is_full_complete: bool) -> bool {
        let (_, last) = extract_last_word(&self.arena, &self.composition, &self.input_method.keys);
        is_valid(&self.arena, &last, input_is_full_complete)
    }

    pub fn get_processed_string(&self, mode: Mode) -> String {
        if mode.contains(Mode::FULL_TEXT) {
            return crate::flattener::flatten(&self.arena, &self.composition, mode);
        }
        if mode.contains(Mode::PUNCTUATION) {
            let (_, last) = extract_last_word_with_punctuation_marks(
                &self.arena,
                &self.composition,
                &self.input_method.keys,
            );
            return crate::flattener::flatten(&self.arena, &last, Mode::VIETNAMESE);
        }
        let (_, last) = extract_last_word(&self.arena, &self.composition, &self.input_method.keys);
        crate::flattener::flatten(&self.arena, &last, mode)
    }

    pub fn can_process_key(&self, key: char) -> bool {
        can_process_key(key, &self.input_method.keys)
    }

    fn get_applicable_rules(&self, key: char) -> Vec<Rule> {
        let lower = simple_lower(key);
        self.input_method
            .rules
            .iter()
            .filter(|rule| rule.key == lower)
            .cloned()
            .collect()
    }

    fn new_composition(&mut self, comp: &[Node], key: char, is_upper_case: bool) -> Vec<Node> {
        let (previous, last) = extract_last_syllable(&self.arena, comp);
        let generated = self.generate_transformations(&last, key, is_upper_case);
        let mut out = previous;
        out.extend_from_slice(&last);
        out.extend(generated);
        out
    }

    fn generate_transformations(
        &mut self,
        comp: &[Node],
        key: char,
        is_upper_case: bool,
    ) -> Vec<Node> {
        let applicable = self.get_applicable_rules(key);
        let mut transformations = generate_transformations(
            &mut self.arena,
            comp,
            &applicable,
            self.flags,
            key,
            is_upper_case,
        );

        if transformations.is_empty() {
            transformations = generate_fallback_transformations(
                &mut self.arena,
                &applicable,
                key,
                is_upper_case,
            );
            let mut new_comp: Vec<Node> = comp.to_vec();
            new_comp.extend_from_slice(&transformations);
            if let Some(virtual_trans) = self.apply_uow_shortcut(&new_comp) {
                transformations.push(virtual_trans);
            }
        }

        let mut joined: Vec<Node> = comp.to_vec();
        joined.extend_from_slice(&transformations);
        let refresh = self.refresh_last_tone_target(&joined);
        transformations.extend(refresh);
        transformations
    }

    fn apply_uow_shortcut(&mut self, syllable: &[Node]) -> Option<Node> {
        let str_ = crate::flattener::flatten(
            &self.arena,
            syllable,
            Mode::TONE_LESS | Mode::LOWER_CASE,
        );
        if self.input_method.super_keys.is_empty() || !matches_uoh_tail(&str_) {
            return None;
        }
        let super_key = self.input_method.super_keys[0];
        let applicable = self.get_applicable_rules(super_key);
        let (target, missing_rule) = find_target(&mut self.arena, syllable, &applicable, self.flags)?;
        let mut missing_rule = missing_rule;
        missing_rule.key = '\0';
        Some(alloc(
            &mut self.arena,
            Transformation::probe(target, missing_rule),
        ))
    }

    fn refresh_last_tone_target(&mut self, comp: &[Node]) -> Vec<Node> {
        if self.flags.contains(Flags::FREE_TONE_MARKING) && is_valid(&self.arena, comp, false) {
            return refresh_last_tone_target(
                &mut self.arena,
                comp,
                self.flags.contains(Flags::STD_TONE_STYLE),
            );
        }
        Vec::new()
    }

    pub fn process_string(&mut self, text: &str, mode: Mode) {
        for key in text.chars() {
            self.process_key(key, mode);
        }
    }

    pub fn process_key(&mut self, key: char, mode: Mode) {
        let lower_key = simple_lower(key);
        let is_upper_case = key.is_uppercase();
        if mode.contains(Mode::ENGLISH) || !self.can_process_key(lower_key) {
            let node = alloc(&mut self.arena, Transformation::appending(lower_key, is_upper_case));
            if mode.contains(Mode::IN_REVERSE_ORDER) {
                self.composition.insert(0, node);
            } else {
                self.composition.push(node);
            }
            return;
        }
        let comp = std::mem::take(&mut self.composition);
        self.composition = self.new_composition(&comp, lower_key, is_upper_case);
    }

    pub fn restore_last_word(&mut self, to_vietnamese: bool) {
        let comp = std::mem::take(&mut self.composition);
        let (previous, last_comb) = extract_last_word(&self.arena, &comp, &self.input_method.keys);
        if last_comb.is_empty() {
            self.composition = comp;
            return;
        }
        let mut out = previous;
        if !to_vietnamese {
            let broken = break_composition(&mut self.arena, &last_comb);
            out.extend(broken);
        } else {
            let mut new_comp: Vec<Node> = Vec::new();
            for &idx in &last_comb {
                let (key, upper) = (self.arena[idx].rule.key, self.arena[idx].is_upper_case);
                new_comp = self.new_composition(&new_comp, key, upper);
            }
            out.extend(new_comp);
        }
        self.composition = out;
    }

    pub fn reset(&mut self) {
        self.composition.clear();
        // Upstream relies on the GC to drop the transient probe nodes; here the
        // arena is only truncated at reset, which is when the word ends.
        self.arena.clear();
    }

    pub fn remove_last_char(&mut self, refresh: bool) {
        let Some(last_appending) = find_last_appending_trans(&self.arena, &self.composition) else {
            return;
        };
        let last_key = self.arena[last_appending].rule.key;
        if !self.can_process_key(last_key) {
            self.composition.pop();
            return;
        }
        let comp = std::mem::take(&mut self.composition);
        let (previous, last_comb) = extract_last_word(&self.arena, &comp, &self.input_method.keys);
        let mut new_comb: Vec<Node> = last_comb
            .iter()
            .copied()
            .filter(|&idx| {
                let trans = &self.arena[idx];
                trans.target != Some(last_appending) && idx != last_appending
            })
            .collect();
        if refresh {
            let extra = self.refresh_last_tone_target(&new_comb);
            new_comb.extend(extra);
        }
        let mut out = previous;
        out.extend(new_comb);
        self.composition = out;
    }
}
