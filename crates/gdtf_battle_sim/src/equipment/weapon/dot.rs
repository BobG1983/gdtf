use std::num::NonZeroU8;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use super::DamageType;

/// `#[serde(transparent)]` so a weapon's authored [`DotProfile`] `.ron` names it as a bare
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DotDamage(u16);

impl DotDamage {
        #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// private inner + derived [`Deref`]; `#[serde(transparent)]` so a weapon's authored
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DotTurns(NonZeroU8);

impl DotTurns {
        #[must_use]
    pub const fn new(turns: NonZeroU8) -> Self {
        Self(turns)
    }

                    #[must_use]
    pub const fn decremented(self) -> Option<Self> {
        match NonZeroU8::new(self.0.get() - 1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

/// the armed entity (the `dot:` field on [`WeaponSpec`](super::WeaponSpec), `#[serde(default)]`
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DotProfile {
        pub damage:      DotDamage,
        pub damage_type: DamageType,
        pub turns:       DotTurns,
}

impl DotProfile {
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
            damage:      DotDamage::default(),
            damage_type: DamageType::default(),
            turns:       DotTurns::new(NonZeroU8::MIN),
        }
    }
}

/// The battle-state side of the DOT model: a `#[derive(Component)]` the fire path attaches
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot {
            pub remaining_turns: DotTurns,
        pub per_turn_damage: DotDamage,
        pub damage_type:     DamageType,
}

impl Dot {
            #[must_use]
    pub const fn from_profile(profile: DotProfile) -> Self {
        Self {
            remaining_turns: profile.turns,
            per_turn_damage: profile.damage,
            damage_type:     profile.damage_type,
        }
    }

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
