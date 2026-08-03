//! [`GangerSpawn`] — the combined roster-plus-placement authoring helper and its
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GangerSpawn {
        pub at:         CellLevel,
                    /// situation `.ron` as a bare string ([`GangerName`] is `#[serde(transparent)]`).
    pub name:       GangerName,
        pub faction:    Faction,
        pub facing:     Facing,
        pub stance:     Stance,
        pub aiming:     Aiming,
        pub life_state: LifeState,
            /// bare scalar ([`Speed`] is `#[serde(transparent)]`).
    pub speed:      Speed,
            /// scalar ([`Aim`] is `#[serde(transparent)]`).
    pub aim:        Aim,
        /// Fight term. Authored as a bare scalar ([`Strength`] is `#[serde(transparent)]`).
    pub strength:   Strength,
                pub toughness:  Toughness,
            /// `#[serde(transparent)]`).
    pub reflexes:   Reflexes,
            /// ([`Cool`] is `#[serde(transparent)]`).
    pub cool:       Cool,
            /// ([`Grit`] is `#[serde(transparent)]`).
    pub grit:       Grit,
                pub luck:       Luck,
                                            pub armor:      crate::armor::ArmorName,
                                                    pub weapon:     WeaponName,
}

impl GangerSpawn {
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
            name:         self.name.clone(),
            speed:        self.speed,
            aim:          self.aim,
            strength:     self.strength,
            toughness:    self.toughness,
            reflexes:     self.reflexes,
            cool:         self.cool,
            grit:         self.grit,
            luck:         self.luck,
            armor:        self.armor.clone(),
            weapon:       self.weapon.clone(),
            melee_weapon: None,
        };
        (placed, member)
    }
}
