use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Resource, Default, Debug)]
pub struct GangerSprites {
            map: HashMap<Entity, Entity>,
}

impl GangerSprites {
        pub(super) fn insert(&mut self, sim: Entity, sprite: Entity) {
        self.map.insert(sim, sprite);
    }

        #[must_use]
    pub fn sprite_for(&self, sim: Entity) -> Option<Entity> {
        self.map.get(&sim).copied()
    }

            pub(super) fn remove(&mut self, sim: Entity) -> Option<Entity> {
        self.map.remove(&sim)
    }

        #[must_use]
    pub fn contains(&self, sim: Entity) -> bool {
        self.map.contains_key(&sim)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GangerSprite {
        pub entity: Entity,
}
