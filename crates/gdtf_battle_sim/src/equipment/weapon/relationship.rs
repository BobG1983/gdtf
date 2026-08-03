use bevy::{
    ecs::relationship::RelationshipTarget,
    prelude::{Component, Entity},
};

/// (`#[relationship(relationship_target = Wields)]`): inserting it on a weapon entity
/// The inner field is **private** (no-bare-types rule 5): the `#[relationship]` derive's
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[relationship(relationship_target = Wields)]
pub struct WieldedBy(Entity);

impl WieldedBy {
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

/// A Bevy [`RelationshipTarget`] (`#[relationship_target(relationship = WieldedBy,
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = WieldedBy, linked_spawn)]
pub struct Wields(Vec<Entity>);

impl Wields {
                                                                            #[must_use]
    pub fn weapon(&self) -> Option<Entity> {
        self.iter().next()
    }

                                                                    #[must_use]
    pub fn ranged_weapon(&self, is_melee: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| !is_melee(entity))
    }

                                            #[must_use]
    pub fn melee_weapon(&self, is_melee: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| is_melee(entity))
    }

                                                                        #[must_use]
    pub fn mounted_weapon(&self, is_mounted: impl Fn(Entity) -> bool) -> Option<Entity> {
        self.iter().find(|&entity| is_mounted(entity))
    }

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
