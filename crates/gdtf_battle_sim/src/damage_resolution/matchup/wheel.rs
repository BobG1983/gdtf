use bevy::prelude::Deref;

use crate::{armor::ArmorType, tuning::CombatTuning, weapon::DamageType};

pub(super) const WHEEL_NODE_COUNT: u8 = 7;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WheelNode(u8);

impl WheelNode {
                #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index % WHEEL_NODE_COUNT)
    }

                                #[must_use]
    pub const fn strong_against(self) -> [Self; 3] {
        [
            Self::new(self.0 + 3),
            Self::new(self.0 + 5),
            Self::new(self.0 + 6),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matchup {
            Favorable,
            Neutral,
            Resisted,
}

impl Matchup {
            pub const ALL: [Self; 3] = [Self::Favorable, Self::Neutral, Self::Resisted];
}

impl DamageType {
                                #[must_use]
    pub fn node(self) -> WheelNode {
        node_index(&Self::ALL, self)
    }
}

impl ArmorType {
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

/// `#[serde(transparent)]` so it parses a bare RON scalar. The magnitudes are
#[derive(Deref, Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct MatchupMultiplier(f32);

impl MatchupMultiplier {
            #[must_use]
    pub const fn new(multiplier: f32) -> Self {
        Self(multiplier)
    }
}

#[must_use]
pub const fn matchup_multiplier(matchup: Matchup, tuning: &CombatTuning) -> MatchupMultiplier {
    let multipliers = tuning.matchup_multipliers;
    match matchup {
        Matchup::Favorable => multipliers.favorable,
        Matchup::Neutral => multipliers.neutral,
        Matchup::Resisted => multipliers.resisted,
    }
}
