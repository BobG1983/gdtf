//! Field lifetime: turn counts and permanent.

use std::num::NonZeroU8;

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{ApplyFieldEffect, FieldExpired};

/// Non-zero remaining turns for a field.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct FieldTurns(NonZeroU8);

impl FieldTurns {
    /// Wrap a non-zero turn count.
    #[must_use]
    pub const fn new(turns: NonZeroU8) -> Self {
        Self(turns)
    }

    /// One less turn, or `None` when this was the last.
    #[must_use]
    pub const fn decremented(self) -> Option<Self> {
        match NonZeroU8::new(self.0.get() - 1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

/// How long a field lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum FieldDuration {
    /// Finite turn count.
    Turns(FieldTurns),
    /// Never expires.
    Permanent,
}

/// Applies duration behaviour to a placed field.
pub struct ApplyDuration {
    duration: FieldDuration,
}

impl ApplyDuration {
    /// Build the applicator.
    #[must_use]
    pub const fn new(duration: FieldDuration) -> Self {
        Self { duration }
    }
}

impl ApplyFieldEffect for ApplyDuration {
    fn initial_countdown(&self) -> Option<FieldTurns> {
        match self.duration {
            FieldDuration::Turns(turns) => Some(turns),
            FieldDuration::Permanent => None,
        }
    }

    fn count_down_one_turn(&self, remaining: &mut Option<FieldTurns>) -> FieldExpired {
        FieldExpired::new(match self.duration {
            FieldDuration::Turns(_) => match (*remaining).and_then(FieldTurns::decremented) {
                Some(next) => {
                    *remaining = Some(next);
                    false
                }
                None => true,
            },
            FieldDuration::Permanent => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyDuration, FieldDuration};
    use crate::{effects::fields::ApplyFieldEffect, test_support::field_turns};

    #[test]
    fn an_authored_zero_turn_duration_fails_deserialization() {
        let parsed = ron::from_str::<FieldDuration>("Turns(0)");
        assert!(
            parsed.is_err(),
            "an authored zero-turn field duration must FAIL the parse (got {parsed:?})",
        );
    }

    #[test]
    fn an_authored_positive_turn_count_parses_unchanged() {
        let parsed = ron::from_str::<FieldDuration>("Turns(3)");
        assert_eq!(
            parsed.ok(),
            Some(FieldDuration::Turns(field_turns(3))),
            "a positive authored turn count must parse to exactly itself",
        );
    }

    #[test]
    fn the_initial_countdown_is_the_authored_turns_count() {
        assert_eq!(
            ApplyDuration::new(FieldDuration::Turns(field_turns(3))).initial_countdown(),
            Some(field_turns(3)),
            "a Turns field starts at its authored count"
        );
        assert_eq!(
            ApplyDuration::new(FieldDuration::Permanent).initial_countdown(),
            None,
            "a Permanent field carries no countdown"
        );
    }

    #[test]
    fn a_turns_field_counts_down_and_expires_on_its_last_round() {
        let lifetime = ApplyDuration::new(FieldDuration::Turns(field_turns(2)));
        let mut remaining = lifetime.initial_countdown();
        assert!(
            !*lifetime.count_down_one_turn(&mut remaining),
            "2 → 1: not yet expired"
        );
        assert_eq!(
            remaining,
            Some(field_turns(1)),
            "the countdown decremented one turn"
        );
        assert!(
            *lifetime.count_down_one_turn(&mut remaining),
            "1 → expired: the round that spends the last turn expires the placement"
        );
    }

    #[test]
    fn a_permanent_field_never_counts_down_and_never_expires() {
        let lifetime = ApplyDuration::new(FieldDuration::Permanent);
        let mut remaining = lifetime.initial_countdown();
        for _ in 0..10 {
            assert!(
                !*lifetime.count_down_one_turn(&mut remaining),
                "a Permanent field never expires"
            );
        }
        assert_eq!(
            remaining, None,
            "a Permanent field never touches the countdown"
        );
    }
}
