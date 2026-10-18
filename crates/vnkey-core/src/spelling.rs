//! Phonotactic spell checking.


// Boolean structure mirrors the upstream Go expressions line for line;
// simplifying it would break the 1:1 diff against `vnkey-core`.
#![allow(clippy::nonminimal_bool, clippy::collapsible_if, clippy::if_same_then_else)]
use crate::utils::add_mark_to_toneless_char;

/// Onset groups. Every row is a valid CV class; the matrix below says which
/// vowel rows each onset may combine with.
const FIRST_CONSONANT_SEQS: [&str; 5] = [
    "b d đ g gh m n nh p ph r s t tr v z",
    "c h k kh qu th",
    "ch gi l ng ngh x",
    "đ l",
    "h",
];

const VOWEL_SEQS: [&str; 8] = [
    "ê i ua uê uy y",
    "a iê oa uyê yê",
    "â ă e o oo ô ơ oe u ư uâ uô ươ",
    "oă",
    "uơ",
    "ai ao au âu ay ây eo êu ia iêu iu oai oao oay oeo oi ôi ơi ưa uây ui ưi uôi ươi ươu ưu uya uyu yêu",
    "ă",
    "i",
];

const LAST_CONSONANT_SEQS: [&str; 5] = ["ch nh", "c ng", "m n p t", "k", "c"];

const CV_MATRIX: [&[usize]; 5] = [
    &[0, 1, 2, 5],
    &[0, 1, 2, 3, 4, 5],
    &[0, 1, 2, 3, 5],
    &[6],
    &[7],
];

const VC_MATRIX: [&[usize]; 8] = [
    &[0, 2],
    &[0, 1, 2],
    &[1, 2],
    &[1, 2],
    &[],
    &[],
    &[3],
    &[4],
];

/// Indexes of the sequences in `seq` that match `input`.
///
/// `input_is_full` requires an exact-length match, `input_is_complete` forbids
/// matching against the toneless form of a marked candidate.
fn lookup(seq: &[&str], input: &str, input_is_full: bool, input_is_complete: bool) -> Vec<usize> {
    let mut ret = Vec::new();
    let input_chars: Vec<char> = input.chars().collect();
    let input_len = input_chars.len();

    for (index, row) in seq.iter().enumerate() {
        for canvas in row.split(' ') {
            let canvas_chars: Vec<char> = canvas.chars().collect();
            if canvas_chars.len() < input_len {
                continue;
            }
            if input_is_full && canvas_chars.len() > input_len {
                continue;
            }
            let mut is_match = true;
            for (k, &ic) in input_chars.iter().enumerate() {
                if ic != canvas_chars[k]
                    && !(!input_is_complete && add_mark_to_toneless_char(canvas_chars[k], 0) == ic)
                {
                    is_match = false;
                    break;
                }
            }
            if is_match {
                ret.push(index);
                break;
            }
        }
    }
    ret
}

fn is_valid_cv(fc_indexes: &[usize], vo_indexes: &[usize]) -> bool {
    for &fc in fc_indexes {
        for &c in CV_MATRIX[fc] {
            if vo_indexes.contains(&c) {
                return true;
            }
        }
    }
    false
}

fn is_valid_vc(vo_indexes: &[usize], lc_indexes: &[usize]) -> bool {
    for &vo in vo_indexes {
        for &c in VC_MATRIX[vo] {
            if lc_indexes.contains(&c) {
                return true;
            }
        }
    }
    false
}

/// The full CV(C) check. `fc`, `vo`, `lc` are the flattened onset, nucleus and
/// coda of a syllable (tone and mark stripped upstream).
pub fn is_valid_cvc(fc: &str, vo: &str, lc: &str, input_is_full_complete: bool) -> bool {
    let mut fc_indexes = Vec::new();
    let mut vo_indexes = Vec::new();
    let mut lc_indexes = Vec::new();

    if !fc.is_empty() {
        fc_indexes = lookup(
            &FIRST_CONSONANT_SEQS,
            fc,
            input_is_full_complete || !vo.is_empty(),
            true,
        );
        if fc_indexes.is_empty() {
            return false;
        }
    }
    if !vo.is_empty() {
        vo_indexes = lookup(
            &VOWEL_SEQS,
            vo,
            input_is_full_complete || !lc.is_empty(),
            input_is_full_complete,
        );
        if vo_indexes.is_empty() {
            return false;
        }
    }
    if !lc.is_empty() {
        lc_indexes = lookup(&LAST_CONSONANT_SEQS, lc, input_is_full_complete, true);
        if lc_indexes.is_empty() {
            return false;
        }
    }

    if vo_indexes.is_empty() {
        // onset only
        return !fc_indexes.is_empty();
    }
    if !fc_indexes.is_empty() {
        // onset + nucleus
        let ret = is_valid_cv(&fc_indexes, &vo_indexes);
        if !ret || lc_indexes.is_empty() {
            return ret;
        }
    }
    if !lc_indexes.is_empty() {
        is_valid_vc(&vo_indexes, &lc_indexes)
    } else {
        true
    }
}
