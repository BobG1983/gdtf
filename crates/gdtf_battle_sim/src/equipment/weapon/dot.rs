//! Damage-over-time profile and live DOT component.

use std::num::NonZeroU8;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use super::DamageType;

/// Damage applied each DOT tick.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DotDamage(u16);

impl DotDamage {
    /// Wrap a damage value.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// Remaining turns of DOT (never zero).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DotTurns(NonZeroU8);

impl DotTurns {
    /// Wrap a positive turn count.
    #[must_use]
    pub const fn new(turns: NonZeroU8) -> Self {
        Self(turns)
    }

    /// One less turn, or `None` when expired.
    #[must_use]
    pub const fn decremented(self) -> Option<Self> {
        match NonZeroU8::new(self.0.get() - 1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

/// Authored DOT on a weapon (optional).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DotProfile {
    /// Per-turn damage.
    pub damage: DotDamage,
    /// Damage channel.
    pub damage_type: DamageType,
    /// Duration in turns.
    pub turns: DotTurns,
}

impl DotProfile {
    /// Build a profile.
    #[must_use]
    pub const fn new(damage: DotDamage, damage_type: DamageType, turns: DotTurns) -> Self {
        Self {
            damage,
            damage_type,
            turns,
        }
    }
}

impl Default for DotProfile {
    fn default() -> Self {
        Self {
            damage: DotDamage::default(),
            damage_type: DamageType::default(),
            turns: DotTurns::new(NonZeroU8::MIN),
        }
    }
}

/// Live DOT on a target entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot {
    /// Turns left.
    pub remaining_turns: DotTurns,
    /// Damage per turn.
    pub per_turn_damage: DotDamage,
    /// Damage channel.
    pub damage_type: DamageType,
}

impl Dot {
    /// Start from an authored profile.
    #[must_use]
    pub const fn from_profile(profile: DotProfile) -> Self {
        Self {
            remaining_turns: profile.turns,
            per_turn_damage: profile.damage,
            damage_type: profile.damage_type,
        }
    }

    /// Replace with a fresh profile.
    pub const fn refresh_from(&mut self, profile: DotProfile) {
        *self = Self::from_profile(profile);
    }
}

impl Default for Dot {
    fn default() -> Self {
        Self::from_profile(DotProfile::default())
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU8;

    use super::{DotProfile, DotTurns};

    #[test]
    fn an_authored_zero_turn_count_fails_deserialization() {
        let parsed = ron::from_str::<DotProfile>("(damage: 4, damage_type: Plasma, turns: 0)");
        assert!(
            parsed.is_err(),
            "an authored zero-turn DOT profile must FAIL the parse (got {parsed:?})",
        );
    }

    #[test]
    fn an_authored_positive_turn_count_parses_unchanged() {
        let parsed = ron::from_str::<DotProfile>("(damage: 4, damage_type: Plasma, turns: 3)");
        assert_eq!(
            parsed.ok().map(|profile| profile.turns.get()),
            Some(3),
            "a positive authored turn count must parse to exactly itself",
        );
    }

    #[test]
    fn decremented_counts_down_to_removal_never_zero() {
        let three = NonZeroU8::new(3).map(DotTurns::new);
        let two = three.and_then(DotTurns::decremented);
        let one = two.and_then(DotTurns::decremented);
        let expired = one.and_then(DotTurns::decremented);
        assert_eq!(
            (
                two.map(|t| t.get()),
                one.map(|t| t.get()),
                expired.map(|t| t.get())
            ),
            (Some(2), Some(1), None),
            "decremented() must step 3 → 2 → 1 → None (remove), never storing zero",
        );
    }
}
