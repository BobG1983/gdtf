use std::time::Duration;

use bevy::prelude::*;

pub(super) const FLASH_SECONDS: f32 = 0.4;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FxFlash;

#[derive(Component, Deref, Debug, Clone)]
pub struct FlashTtl(Timer);

impl FlashTtl {
                    #[must_use]
    pub fn new() -> Self {
        Self(Timer::from_seconds(FLASH_SECONDS, TimerMode::Once))
    }

                            pub fn tick(&mut self, delta: Duration) -> bool {
        self.0.tick(delta).is_finished()
    }
}

impl Default for FlashTtl {
        fn default() -> Self {
        Self::new()
    }
}

pub fn expire_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut FlashTtl), With<FxFlash>>,
) {
    let delta = time.delta();
    for (entity, mut ttl) in &mut flashes {
        if ttl.tick(delta) {
            commands.entity(entity).despawn();
        }
    }
}
