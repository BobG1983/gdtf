//! Authored battlefield aggregate.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{battle_map::BattleMap, combatants::SituationCombatants};

/// Complete authored situation for one battle: a map and the sides fighting on it.
#[derive(Debug, Clone, Default, Deserialize, Serialize, TypePath)]
#[serde(default)]
pub struct Situation {
    /// The battlefield.
    pub map:        BattleMap,
    /// The sides fighting on it.
    pub combatants: SituationCombatants,
}

impl Situation {
    /// Empty situation with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
