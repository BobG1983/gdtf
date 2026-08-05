//! Bevy relationships between wearer and armor piece entities.

use bevy::{
    ecs::{
        query::{QueryData, With},
        relationship::RelationshipTarget,
        system::SystemParam,
    },
    prelude::{Component, Entity, Query},
};

use super::stats::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

/// Armor piece → wearer (`relationship_target = Wears`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Wears)]
pub struct WornBy(Entity);

impl WornBy {
    /// Point at the wearer entity.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

impl Default for WornBy {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Wearer → armor pieces. Target for [`WornBy`].
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = WornBy, linked_spawn)]
pub struct Wears(Vec<Entity>);

impl Wears {
    /// Iterate piece entities.
    pub fn pieces(&self) -> impl Iterator<Item = Entity> + '_ {
        self.iter()
    }
}

/// Mutable query data for one worn piece.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct PieceArmorMut {
    /// Body part.
    pub part:       &'static BodyPart,
    /// Floor.
    pub floor:      &'static ArmorFloor,
    /// Protection.
    pub protection: &'static ArmorProtection,
    /// Integrity (mutable for wear).
    pub integrity:  &'static mut ArmorIntegrity,
    /// Hardness.
    pub hardness:   &'static ArmorHardness,
    /// Type.
    pub armor_type: &'static ArmorType,
}

/// The armor sets combatants wear and the individual pieces a hit damages.
#[derive(SystemParam)]
pub struct WornArmor<'w, 's> {
    /// Worn armor set on each combatant.
    pub wears:  Query<'w, 's, &'static Wears>,
    /// Individual worn armor pieces.
    pub pieces: Query<'w, 's, PieceArmorMut, With<WornBy>>,
}
