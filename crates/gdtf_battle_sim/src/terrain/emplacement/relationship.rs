//! Bevy relationship between an emplacement and the ganger riding it.

use bevy::{
    ecs::relationship::RelationshipTarget,
    prelude::{Component, Entity},
};

/// Emplacement → its occupant (`relationship_target = Mounted`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Mounted)]
pub struct MountedBy(Entity);

impl MountedBy {
    /// Point at the ganger manning this emplacement.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

impl Default for MountedBy {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Ganger → the emplacement it rides. Target for [`MountedBy`].
#[derive(Component, Debug)]
#[relationship_target(relationship = MountedBy)]
pub struct Mounted(Entity);

impl Mounted {
    /// The emplacement this ganger rides, absent while the record is being torn down.
    #[must_use]
    pub fn emplacement(&self) -> Option<Entity> {
        self.iter().next()
    }
}
