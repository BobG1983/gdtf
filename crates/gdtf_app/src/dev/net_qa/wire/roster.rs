//! Roster cards on the wire, mirroring what the stat block draws.

use bevy::prelude::Deref;
use gdtf_battle_sim::prelude::Faction;
use serde::{Deserialize, Serialize};

use super::{
    act_payload::StanceNet,
    cell::CellLevelNet,
    token::GangerToken,
    vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
    wound::{InjuryNet, WoundNet},
};

/// Ganger display name.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangerNameNet(String);

impl GangerNameNet {
    /// Build from a display name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Gang index a ganger fights for.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FactionNet(u8);

impl FactionNet {
    /// Build from a gang index.
    #[must_use]
    pub const fn new(gang: u8) -> Self {
        Self(gang)
    }

    /// Mirror the sim's faction marker.
    #[must_use]
    pub fn from_sim(faction: Faction) -> Self {
        Self(*faction)
    }
}

/// One roster card, carrying the values the stat block is drawing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GangerCardNet {
    /// Opaque ganger identity.
    pub token:        GangerToken,
    /// Cell the sprite is drawn on, which lags the sim while an act plays out.
    pub at:           CellLevelNet,
    /// Display name, absent for an unnamed ganger.
    pub name:         Option<GangerNameNet>,
    /// Gang index.
    pub faction:      FactionNet,
    /// Posture.
    pub stance:       StanceNet,
    /// Remaining time units.
    pub tu:           TuNet,
    /// Time units at the start of a turn.
    pub tu_max:       TuMaxNet,
    /// Current hit points.
    pub hp:           HpNet,
    /// Hit points at full health.
    pub hp_max:       HpMaxNet,
    /// Wounds still standing.
    pub wounds:       WoundsNet,
    /// Wound count at full health.
    pub wounds_max:   WoundsMaxNet,
    /// Wounds this ganger has taken.
    pub wounds_taken: Vec<WoundNet>,
    /// Lasting injuries.
    pub injuries:     Vec<InjuryNet>,
}
