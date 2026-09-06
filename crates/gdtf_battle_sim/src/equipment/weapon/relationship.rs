//! Bevy relationships between wielder and weapon entities.

use bevy::{
    ecs::{relationship::RelationshipTarget, system::SystemParam},
    prelude::{Changed, Component, Entity, Query, RemovedComponents},
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

    /// The gun on the wielder's own back: the first weapon that is neither melee nor mounted.
    #[must_use]
    pub fn carried_ranged_weapon(
        &self,
        is_mounted: impl Fn(Entity) -> bool,
        is_melee: impl Fn(Entity) -> bool,
    ) -> Option<Entity> {
        self.iter()
            .find(|&entity| !is_mounted(entity) && !is_melee(entity))
    }

    /// The weapon that fires: the mounted one if there is one, else the ranged one.
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

/// Whether any wielder picked up or lost a weapon since the reading system last ran.
#[derive(SystemParam)]
pub struct WieldsChanged<'w, 's> {
    wielded:   Query<'w, 's, (), Changed<Wields>>,
    unwielded: RemovedComponents<'w, 's, WieldedBy>,
}

impl WieldsChanged<'_, '_> {
    /// Whether a weapon was wielded or unwielded since the last read. Drains what it reads.
    pub fn any(&mut self) -> bool {
        // Drain first: the reader's cursor only advances on read, so a skipped read repeats.
        let unwielded = self.unwielded.read().count() > 0;
        unwielded || !self.wielded.is_empty()
    }
}
