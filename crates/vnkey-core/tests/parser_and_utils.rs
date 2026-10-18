//! Parser and character-table tests.

use vnkey_core::rules_parser::{parse_rules, parse_toneless_rules};
use vnkey_core::utils::{
    add_mark_to_char, add_tone_to_char, find_tone_from_char, is_vowel,
};
use vnkey_core::{EffectType, Mark, Tone};

#[test]
fn parse_tone_rules() {
    let rules = parse_rules('z', "XoaDauThanh");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].effect_type, Some(EffectType::Tone));
    assert_eq!(rules[0].tone(), Tone::None);

    let rules = parse_rules('x', "DauNga");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].effect_type, Some(EffectType::Tone));
    assert_eq!(rules[0].tone(), Tone::Tilde);
}

#[test]
fn parse_toneless_rules_dash() {
    let rules = parse_toneless_rules('d', "D_Đ");
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].effect_type, Some(EffectType::Mark));
    assert_eq!(rules[0].effect, Mark::Dash as u8);
    assert_eq!(rules[0].effect_on, 'd');
}

#[test]
fn parse_toneless_rules_appending() {
    let rules = parse_toneless_rules('{', "_Ư");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].effect_type, Some(EffectType::Appending));
    assert_eq!(rules[0].effect_on, 'Ư');
}

#[test]
fn parse_toneless_rules_uoa() {
    let rules = parse_toneless_rules('w', "UOA_ƯƠĂ");
    assert_eq!(rules.len(), 33);

    assert_eq!(rules[0].effect_type, Some(EffectType::Mark));
    assert_eq!(rules[0].mark(), Mark::Horn);
    assert_eq!(rules[0].effect_on, 'u');

    assert_eq!(rules[7].effect_type, Some(EffectType::Mark));
    assert_eq!(rules[7].mark(), Mark::Horn);
    assert_eq!(rules[7].effect_on, 'o');

    assert_eq!(rules[20].effect_type, Some(EffectType::Mark));
    assert_eq!(rules[20].mark(), Mark::Breve);
    assert_eq!(rules[20].effect_on, 'a');
}

#[test]
fn parse_toneless_rules_uoa_with_tail() {
    let rules = parse_toneless_rules('w', "UOA_ƯƠĂ__Ư");
    assert_eq!(rules.len(), 34);

    assert_eq!(rules[20].effect_type, Some(EffectType::Mark));
    assert_eq!(rules[20].mark(), Mark::Breve);
    assert_eq!(rules[20].effect_on, 'a');

    assert_eq!(rules[33].effect_type, Some(EffectType::Appending));
    assert_eq!(rules[33].effect_on, 'ư');
}

#[test]
fn append_rule() {
    let rules = parse_toneless_rules('[', "__ươ");
    assert_eq!(rules.len(), 1);
    let appended = &rules[0].appended_rules;
    assert_eq!(appended.len(), 1);
    assert_eq!(appended[0].effect_type, Some(EffectType::Appending));
    assert_eq!(appended[0].effect_on, 'ơ');

    let rules = parse_toneless_rules('{', "__ƯƠ");
    assert_eq!(rules.len(), 1);
    let appended = &rules[0].appended_rules;
    assert_eq!(appended.len(), 1);
    assert_eq!(appended[0].effect_type, Some(EffectType::Appending));
    assert_eq!(appended[0].effect_on, 'Ơ');
}

#[test]
fn is_vowel_check() {
    assert!(is_vowel('a'));
    assert!(is_vowel('á'));
    assert!(!is_vowel('b'));
    for v in "aàáảãạăằắẳẵặâầấẩẫậeèéẻẽẹêềếểễệiìíỉĩịoòóỏõọôồốổỗộơờớởỡợuùúủũụưừứửữựyỳýỷỹỵ".chars() {
        assert!(is_vowel(v), "{v} should be a vowel");
    }
}

#[test]
fn get_tone_from_char() {
    assert_eq!(find_tone_from_char('e'), Tone::None);
    assert_eq!(find_tone_from_char('è'), Tone::Grave);
    assert_eq!(find_tone_from_char('é'), Tone::Acute);
    assert_eq!(find_tone_from_char('ẽ'), Tone::Tilde);
    assert_eq!(find_tone_from_char('ẻ'), Tone::Hook);
    assert_eq!(find_tone_from_char('ạ'), Tone::Dot);
}

#[test]
fn add_tone_to_char_applies() {
    assert_eq!(add_tone_to_char('a', Tone::Dot as u8), 'ạ');
    assert_eq!(add_tone_to_char('y', 0), 'y');
    assert_eq!(add_mark_to_char('y', 0), 'y');
}

#[test]
fn add_mark_to_char_keeps_tone() {
    assert_eq!(add_mark_to_char('ạ', Mark::Breve as u8), 'ặ');
}
