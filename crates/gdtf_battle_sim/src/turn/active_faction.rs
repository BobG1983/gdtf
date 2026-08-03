//! Which faction currently acts.

use bevy::prelude::{Deref, Resource};

use crate::ganger::Faction;

const TEAM_COUNT: u8 = 2;

/// Currently active faction (two-team cycle).
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveFaction(Faction);

impl ActiveFaction {
    /// Set the active faction.
    #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }

    /// Advance to the next team (wraps at [`TEAM_COUNT`]).
    pub fn advance(&mut self) {
        let next = (*self.0).wrapping_add(1) % TEAM_COUNT;
        self.0 = Faction::new(next);
    }
}
