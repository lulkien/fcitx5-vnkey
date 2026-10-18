//! Rule and input-method types.

/// Tone marks. The discriminant is the index into a vowel row, see `tables`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tone {
    #[default]
    None = 0,
    Grave = 1,
    Acute = 2,
    Hook = 3,
    Tilde = 4,
    Dot = 5,
}

impl Tone {
    pub fn from_u8(value: u8) -> Tone {
        match value {
            1 => Tone::Grave,
            2 => Tone::Acute,
            3 => Tone::Hook,
            4 => Tone::Tilde,
            5 => Tone::Dot,
            _ => Tone::None,
        }
    }
}

/// Modifier marks (circumflex, breve, horn, dash). The discriminant is the slot
/// inside the mark row for a character family, see `tables`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mark {
    #[default]
    None = 0,
    Hat = 1,
    Breve = 2,
    Horn = 3,
    Dash = 4,
    /// "undo this key" marker: emits the raw keystroke instead of a marked char.
    Raw = 5,
}

impl Mark {
    pub fn from_u8(value: u8) -> Mark {
        match value {
            1 => Mark::Hat,
            2 => Mark::Breve,
            3 => Mark::Horn,
            4 => Mark::Dash,
            5 => Mark::Raw,
            _ => Mark::None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectType {
    Appending,
    Mark,
    Tone,
}

/// One parsed rule: `key` produces `effect` of `effect_type` on `effect_on`,
/// whose plain result is `result`, plus extra characters appended after it.
#[derive(Clone, Debug, Default)]
pub struct Rule {
    pub key: char,
    /// `Tone` or `Mark` discriminant depending on `effect_type`.
    pub effect: u8,
    pub effect_type: Option<EffectType>,
    pub effect_on: char,
    pub result: char,
    pub appended_rules: Vec<Rule>,
}

impl Rule {
    pub fn is_appending(&self) -> bool {
        self.effect_type == Some(EffectType::Appending)
    }

    pub fn is_mark(&self) -> bool {
        self.effect_type == Some(EffectType::Mark)
    }

    pub fn is_tone(&self) -> bool {
        self.effect_type == Some(EffectType::Tone)
    }

    pub fn tone(&self) -> Tone {
        Tone::from_u8(self.effect)
    }

    pub fn mark(&self) -> Mark {
        Mark::from_u8(self.effect)
    }

    /// A rule that only exists to represent a key stroke (`Key == 0` means the
    /// rule was synthesised and has no raw keystroke behind it).
    pub fn is_virtual(&self) -> bool {
        self.key == '\0'
    }
}

/// A parsed input method: the rules for every key, plus derived key lists.
#[derive(Clone, Debug, Default)]
pub struct InputMethod {
    pub name: String,
    pub rules: Vec<Rule>,
    /// Keys whose definition mentions a `uo` cluster (used for the `uow` shortcut).
    pub super_keys: Vec<char>,
    pub tone_keys: Vec<char>,
    pub appending_keys: Vec<char>,
    /// Every key that appears in the definition.
    pub keys: Vec<char>,
}

impl InputMethod {
    pub fn has_key(&self, key: char) -> bool {
        self.keys.contains(&key)
    }
}
