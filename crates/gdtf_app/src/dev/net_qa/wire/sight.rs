//! Whether the selected shooter can see and engage a cell, on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Whether the squad can see the cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanSeeNet(bool);

impl CanSeeNet {
    /// Build from a visibility answer.
    #[must_use]
    pub const fn new(can_see: bool) -> Self {
        Self(can_see)
    }
}

/// Whether the shooter could fire at the cell this turn.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanEngageNet(bool);

impl CanEngageNet {
    /// Build from a firing-arc answer.
    #[must_use]
    pub const fn new(can_engage: bool) -> Self {
        Self(can_engage)
    }
}

/// A sightline answer, or the reason there is none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SightlineNet {
    /// No ganger is selected, so there is no shooter to answer for.
    NoShooter,
    /// Both halves: squad fog for seeing, the shooter's firing arc for engaging.
    Answered {
        /// Whether the squad can see the cell.
        can_see:    CanSeeNet,
        /// Whether the shooter could fire at it this turn.
        can_engage: CanEngageNet,
    },
}
