//! The Gang form's own member-grid values on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Which of a gang member's eight attributes a write names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum GangAttributeNet {
    /// The member's movement attribute.
    Speed,
    /// The member's ranged accuracy.
    Aim,
    /// The member's melee power.
    Strength,
    /// The member's resilience.
    Toughness,
    /// The member's reaction speed.
    Reflexes,
    /// The member's composure under fire.
    Cool,
    /// The member's grit.
    Grit,
    /// The member's luck.
    Luck,
}

impl GangAttributeNet {
    /// Every attribute the member grid draws, for a case that walks them all.
    #[cfg(test)]
    pub(in crate::mcp) const ALL: [Self; 8] = [
        Self::Speed,
        Self::Aim,
        Self::Strength,
        Self::Toughness,
        Self::Reflexes,
        Self::Cool,
        Self::Grit,
        Self::Luck,
    ];
}

/// The number one attribute drag holds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct GangAttributeValueNet(f32);

impl GangAttributeValueNet {
    /// Wrap an attribute value a client sent or a member holds.
    pub(in crate::mcp) const fn new(value: f32) -> Self {
        Self(value)
    }
}
