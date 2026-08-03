//! Bevy relationships between wielder and weapon entities.

use bevy::{
    ecs::relationship::RelationshipTarget,
    prelude::{Component, Entity},
};

/// Weapon → wielder (`relationship_target = Wields`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Wields)]
pub struct WieldedBy(Entity);

impl WieldedBy {
    /// Point at the wielder.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

impl Default for WieldedBy {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Wielder → weapons. Target for [`WieldedBy`].
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = WieldedBy, linked_spawn)]
pub struct Wields(Vec<Entity>);

impl Wields {
    /// First weapon entity, if any.
    #[must_use]
    pub fn weapon(&self) -> Option<Entity> {
        self.iter().next()
    }

    /// First non-melee weapon.
    #[must_use]
    pub fn ranged_weapon(&self, is_melee: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| !is_melee(entity))
    }

    /// First melee weapon.
    #[must_use]
    pub fn melee_weapon(&self, is_melee: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| is_melee(entity))
    }

    /// First mounted weapon.
    #[must_use]
    pub fn mounted_weapon(&self, is_mounted: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| is_mounted(entity))
    }

    /// Prefer mounted, else ranged (for opportunity / reaction fire).
    #[must_use]
    pub fn firing_weapon(
        &self,
        is_mounted: impl Fn(Entity) -> bool,
        is_melee: impl Fn(Entity) -> bool,
    ) -> Option<Entity> {
        self.mounted_weapon(is_mounted)
            .or_else(|| self.ranged_weapon(is_melee))
    }
}
