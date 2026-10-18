//! Bit flags mirroring vnkey-core's `Mode` and engine flags.

use core::ops::{BitOr, BitOrAssign};

/// Output/processing mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Mode(u32);

impl Mode {
    pub const VIETNAMESE: Mode = Mode(1 << 0);
    /// Emit the raw keystrokes instead of the transformed text.
    pub const ENGLISH: Mode = Mode(1 << 1);
    pub const TONE_LESS: Mode = Mode(1 << 2);
    pub const MARK_LESS: Mode = Mode(1 << 3);
    pub const LOWER_CASE: Mode = Mode(1 << 4);
    /// Whole composition rather than just the last word.
    pub const FULL_TEXT: Mode = Mode(1 << 5);
    pub const PUNCTUATION: Mode = Mode(1 << 6);
    pub const IN_REVERSE_ORDER: Mode = Mode(1 << 7);

    pub const fn empty() -> Self {
        Mode(0)
    }

    pub const fn contains(self, other: Mode) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for Mode {
    type Output = Mode;
    fn bitor(self, rhs: Mode) -> Mode {
        Mode(self.0 | rhs.0)
    }
}

impl BitOrAssign for Mode {
    fn bitor_assign(&mut self, rhs: Mode) {
        self.0 |= rhs.0;
    }
}

/// Engine flags.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Flags(u32);

impl Flags {
    pub const FREE_TONE_MARKING: Flags = Flags(1 << 0);
    /// Legacy (oà) tone placement instead of modern (òa).
    pub const STD_TONE_STYLE: Flags = Flags(1 << 1);
    pub const AUTO_CORRECT_ENABLED: Flags = Flags(1 << 2);
    /// `EstdFlags` upstream.
    pub const STD: Flags = Flags(0b111);

    pub const fn empty() -> Self {
        Flags(0)
    }

    pub const fn contains(self, other: Flags) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn set(&mut self, flag: Flags, on: bool) {
        if on {
            self.0 |= flag.0;
        } else {
            self.0 &= !flag.0;
        }
    }
}

impl BitOr for Flags {
    type Output = Flags;
    fn bitor(self, rhs: Flags) -> Flags {
        Flags(self.0 | rhs.0)
    }
}

impl BitOrAssign for Flags {
    fn bitor_assign(&mut self, rhs: Flags) {
        self.0 |= rhs.0;
    }
}
