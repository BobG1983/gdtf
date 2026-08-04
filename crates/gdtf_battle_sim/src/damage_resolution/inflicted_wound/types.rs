//! Single wound and the list of wounds on a combatant.

use bevy::prelude::{Component, Deref};

use crate::{armor::BodyPart, severity::Severity};

/// One wound that has been applied to a body part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InflictedWound {
    /// Severity of the wound.
    pub tier:     Severity,
    /// Body part that was hit.
    pub location: BodyPart,
}

impl InflictedWound {
    /// Build a wound record.
    #[must_use]
    pub const fn new(tier: Severity, location: BodyPart) -> Self {
        Self { tier, location }
    }
}

/// List of wounds currently on a combatant.
/// Appended via [`record`](InflictedWounds::record).
#[derive(Deref, Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct InflictedWounds(Vec<InflictedWound>);

impl InflictedWounds {
    /// Build from an existing list.
    #[must_use]
    pub const fn new(wounds: Vec<InflictedWound>) -> Self {
        Self(wounds)
    }

    /// Append one more wound.
    pub fn record(&mut self, wound: InflictedWound) {
        self.0.push(wound);
    }
}
