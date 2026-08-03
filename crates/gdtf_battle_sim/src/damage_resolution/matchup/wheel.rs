//! Rock-paper-scissors style matchup between damage types and armor types.

use bevy::prelude::Deref;

use crate::{armor::ArmorType, tuning::CombatTuning, weapon::DamageType};

/// Number of nodes on the matchup wheel.
pub(super) const WHEEL_NODE_COUNT: u8 = 7;

/// Index on the seven-node matchup wheel.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WheelNode(u8);

impl WheelNode {
    /// Build a node, wrapping into the wheel range.
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index % WHEEL_NODE_COUNT)
    }

    /// The three nodes this one is strong against.
    #[must_use]
    pub const fn strong_against(self) -> [Self; 3] {
        [
            Self::new(self.0 + 3),
            Self::new(self.0 + 5),
            Self::new(self.0 + 6),
        ]
    }
}

/// Result of comparing a weapon damage type to an armor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matchup {
    /// Weapon is strong against this armor.
    Favorable,
    /// Same node; no advantage either way.
    Neutral,
    /// Armor resists this damage type.
    Resisted,
}

impl Matchup {
    /// All three outcomes.
    pub const ALL: [Self; 3] = [Self::Favorable, Self::Neutral, Self::Resisted];
}

impl DamageType {
    /// Wheel node for this damage type.
    #[must_use]
    pub fn node(self) -> WheelNode {
        node_index(&Self::ALL, self)
    }
}

impl ArmorType {
    /// Wheel node for this armor type.
    #[must_use]
    pub fn node(self) -> WheelNode {
        node_index(&Self::ALL, self)
    }
}

fn node_index<T: PartialEq>(all: &[T; WHEEL_NODE_COUNT as usize], needle: T) -> WheelNode {
    for (index, candidate) in all.iter().enumerate() {
        if *candidate == needle {
            return WheelNode::new(u8::try_from(index).unwrap_or(0));
        }
    }
    WheelNode::new(0)
}

/// Compare a weapon damage type to an armor type.
#[must_use]
pub fn matchup(weapon: DamageType, armor: ArmorType) -> Matchup {
    let weapon_node = weapon.node();
    let armor_node = armor.node();
    if weapon_node == armor_node {
        Matchup::Neutral
    } else if weapon_node.strong_against().contains(&armor_node) {
        Matchup::Favorable
    } else {
        Matchup::Resisted
    }
}

/// Multiplier applied to punch/shred for a given matchup outcome.
///
/// Parsed as a bare RON scalar via `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct MatchupMultiplier(f32);

impl MatchupMultiplier {
    /// Build from a raw multiplier.
    #[must_use]
    pub const fn new(multiplier: f32) -> Self {
        Self(multiplier)
    }
}

/// Look up the configured multiplier for a matchup outcome.
#[must_use]
pub const fn matchup_multiplier(matchup: Matchup, tuning: &CombatTuning) -> MatchupMultiplier {
    let multipliers = tuning.matchup_multipliers;
    match matchup {
        Matchup::Favorable => multipliers.favorable,
        Matchup::Neutral => multipliers.neutral,
        Matchup::Resisted => multipliers.resisted,
    }
}
