use bevy::{
    ecs::{query::QueryData, relationship::RelationshipTarget},
    prelude::{Component, Entity},
};

use super::stats::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

/// (`#[relationship(relationship_target = Wears)]`): inserting
/// The inner field is **private** (no-bare-types rule 5): the `#[relationship]` derive's
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Wears)]
pub struct WornBy(Entity);

impl WornBy {
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

/// A Bevy [`RelationshipTarget`] (`#[relationship_target(relationship = WornBy,
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = WornBy, linked_spawn)]
pub struct Wears(Vec<Entity>);

impl Wears {
                                    pub fn pieces(&self) -> impl Iterator<Item = Entity> + '_ {
        self.iter()
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct PieceArmorMut {
        pub part:       &'static BodyPart,
        pub floor:      &'static ArmorFloor,
        pub protection: &'static ArmorProtection,
        pub integrity:  &'static mut ArmorIntegrity,
        pub hardness:   &'static ArmorHardness,
        pub armor_type: &'static ArmorType,
}
