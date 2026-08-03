//! How many hands a combatant still has free.

use bevy::prelude::Deref;

/// Number of usable hands (0–2).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandsAvailable(u8);

impl HandsAvailable {
    /// Maximum hands a combatant can have.
    pub(crate) const MAX: u8 = 2;

    /// Build, clamping to the maximum.
    #[must_use]
    pub const fn new(hands: u8) -> Self {
        Self(if hands > Self::MAX { Self::MAX } else { hands })
    }
}

impl Default for HandsAvailable {
    fn default() -> Self {
        Self(Self::MAX)
    }
}
