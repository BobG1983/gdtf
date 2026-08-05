//! What the inspect panel shows for one cell, on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::cover::{CoverEntry, HeightBand};
use serde::{Deserialize, Serialize};

use super::roster::GangerCardNet;

/// How much damage cover shrugs off.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HardnessNet(i32);

impl HardnessNet {
    /// Build from a raw hardness value.
    #[must_use]
    pub const fn new(hardness: i32) -> Self {
        Self(hardness)
    }
}

/// How much damage cover soaks for whoever hides behind it.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtectionNet(i32);

impl ProtectionNet {
    /// Build from a raw protection value.
    #[must_use]
    pub const fn new(protection: i32) -> Self {
        Self(protection)
    }
}

/// How tall the cover stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeightBandNet {
    /// Ankle to knee.
    Low,
    /// Knee to chest.
    Mid,
    /// Chest and above.
    High,
}

impl HeightBandNet {
    /// Mirror the sim's height band.
    #[must_use]
    pub const fn from_sim(band: HeightBand) -> Self {
        match band {
            HeightBand::Low => Self::Low,
            HeightBand::Mid => Self::Mid,
            HeightBand::High => Self::High,
        }
    }
}

/// Structural hit points of a cover cell.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CoverHpNet(u32);

impl CoverHpNet {
    /// Build from a raw cover HP value.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }
}

/// The cover block the inspect panel draws for a terrain cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverBlockNet {
    /// Hardness value.
    pub hardness:   HardnessNet,
    /// Protection value.
    pub protection: ProtectionNet,
    /// Height band.
    pub height:     HeightBandNet,
    /// Current structural HP.
    pub hp:         CoverHpNet,
    /// Structural HP when intact.
    pub hp_max:     CoverHpNet,
}

impl CoverBlockNet {
    /// Mirror a sim cover entry.
    #[must_use]
    pub fn from_sim(entry: CoverEntry) -> Self {
        Self {
            hardness:   HardnessNet::new(*entry.armor_hardness),
            protection: ProtectionNet::new(*entry.armor_protection),
            height:     HeightBandNet::from_sim(entry.height_band),
            hp:         CoverHpNet::new(*entry.current_hp),
            hp_max:     CoverHpNet::new(*entry.max_hp),
        }
    }
}

/// The three things the inspect panel can be showing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InspectShownNet {
    /// A squad-visible ganger stands on the cell.
    Ganger(GangerCardNet),
    /// Destructible terrain stands on the cell.
    Cover(CoverBlockNet),
    /// The panel is hidden.
    Nothing,
}
