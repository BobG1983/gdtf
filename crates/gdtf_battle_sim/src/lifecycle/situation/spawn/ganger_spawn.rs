//! Combined roster-plus-placement authoring helper.

use serde::Deserialize;

use super::placed_ganger::{PlacedGanger, Placement};
use crate::{
    ganger::{
        Aim, Aiming, Cool, Facing, Faction, GangMember, GangName, GangerName, Grit, LifeState,
        Luck, Reflexes, Speed, Stance, Strength, Toughness,
    },
    metric::CellLevel,
    weapon::WeaponName,
};

/// One authored ganger with full stats and placement.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GangerSpawn {
    /// Spawn cell.
    pub at: CellLevel,
    /// Display name.
    pub name: GangerName,
    /// Faction index.
    pub faction: Faction,
    /// Facing direction.
    pub facing: Facing,
    /// Stance.
    pub stance: Stance,
    /// Aiming state.
    pub aiming: Aiming,
    /// Life state.
    pub life_state: LifeState,
    /// Speed stat.
    pub speed: Speed,
    /// Aim stat.
    pub aim: Aim,
    /// Strength / fight stat.
    pub strength: Strength,
    /// Toughness.
    pub toughness: Toughness,
    /// Reflexes.
    pub reflexes: Reflexes,
    /// Cool.
    pub cool: Cool,
    /// Grit.
    pub grit: Grit,
    /// Luck.
    pub luck: Luck,
    /// Armor key.
    pub armor: crate::armor::ArmorName,
    /// Primary weapon key.
    pub weapon: WeaponName,
}

impl GangerSpawn {
    /// Split into a placement record and a gang-member catalog entry.
    #[must_use]
    pub fn split(&self, gang: GangName) -> (PlacedGanger, GangMember) {
        let placed = PlacedGanger::new(
            gang,
            self.name.clone(),
            Placement::new(
                self.at,
                self.faction,
                self.facing,
                self.stance,
                self.aiming,
                self.life_state,
            ),
        );
        let member = GangMember {
            name: self.name.clone(),
            speed: self.speed,
            aim: self.aim,
            strength: self.strength,
            toughness: self.toughness,
            reflexes: self.reflexes,
            cool: self.cool,
            grit: self.grit,
            luck: self.luck,
            armor: self.armor.clone(),
            weapon: self.weapon.clone(),
            melee_weapon: None,
        };
        (placed, member)
    }
}
