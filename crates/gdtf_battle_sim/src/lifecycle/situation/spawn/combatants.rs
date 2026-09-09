//! Authored combatant half of a situation: who fights, and on whose side.

use serde::{Deserialize, Serialize};

use super::roster_member::RosterMember;
use crate::ganger::Faction;

/// The sides a battle is fought between, with no map under them.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SituationCombatants {
    /// Roster members without fixed cells (procgen places them).
    pub rosters:        Vec<RosterMember>,
    /// Player's faction index.
    pub player_faction: Faction,
}

impl SituationCombatants {
    /// Empty combatants with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
