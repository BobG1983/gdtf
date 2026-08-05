//! Map from sim ganger entities to their sprite entities.

use bevy::{ecs::system::SystemParam, platform::collections::HashMap, prelude::*};

/// Sim entity → sprite entity lookup.
#[derive(Resource, Default, Debug)]
pub struct GangerSprites {
    map: HashMap<Entity, Entity>,
}

impl GangerSprites {
    pub(super) fn insert(&mut self, sim: Entity, sprite: Entity) {
        self.map.insert(sim, sprite);
    }

    /// Sprite entity for a sim ganger, if spawned.
    #[must_use]
    pub fn sprite_for(&self, sim: Entity) -> Option<Entity> {
        self.map.get(&sim).copied()
    }

    pub(super) fn remove(&mut self, sim: Entity) -> Option<Entity> {
        self.map.remove(&sim)
    }

    /// Whether a sprite exists for this sim entity.
    #[must_use]
    pub fn contains(&self, sim: Entity) -> bool {
        self.map.contains_key(&sim)
    }
}

/// Marker on a ganger sprite, pointing back at the sim entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GangerSprite {
    /// Sim ganger this sprite represents.
    pub entity: Entity,
}

/// Where each sim ganger's sprite currently sits in the world.
#[derive(SystemParam)]
pub struct GangerSpriteWorld<'w, 's> {
    sprites:    Res<'w, GangerSprites>,
    transforms: Query<'w, 's, &'static Transform, With<GangerSprite>>,
}

impl GangerSpriteWorld<'_, '_> {
    /// World position of a sim ganger's drawn sprite, if it has one.
    #[must_use]
    pub fn position_of(&self, sim: Entity) -> Option<Vec3> {
        let sprite = self.sprites.sprite_for(sim)?;
        Some(self.transforms.get(sprite).ok()?.translation)
    }
}
