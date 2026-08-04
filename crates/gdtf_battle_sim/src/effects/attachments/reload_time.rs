//! Reload-time scale attachment effect.

use bevy::prelude::{Deref, EntityWorldMut};
use serde::{Deserialize, Serialize};

use super::ApplyAttachmentEffect;
use crate::magazine::{LoadedRounds, Magazine, ReloadTu};

/// Multiplier applied to magazine reload TU.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReloadTimeScale(f32);

impl ReloadTimeScale {
    /// Wrap a scale factor.
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// Scales the weapon magazine reload cost.
pub struct ApplyReloadTime {
    scale: ReloadTimeScale,
}

impl ApplyReloadTime {
    /// Build the applicator.
    #[must_use]
    pub const fn new(scale: ReloadTimeScale) -> Self {
        Self { scale }
    }
}

impl ApplyAttachmentEffect for ApplyReloadTime {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(magazine) = weapon.get::<Magazine>() else {
            return;
        };
        let scaled = (f32::from(*magazine.reload_tu()) * *self.scale).max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the scaled reload cost is clamped non-negative above and a reload TU is a \
                      small u8 count, so the f32 -> u8 floor cannot truncate meaningfully or \
                      sign-flip (the scale_reload precedent this isolates)"
        )]
        let tu = scaled as u8;
        let rebuilt = Magazine::new(
            LoadedRounds::new(*magazine.rounds()),
            magazine.size(),
            ReloadTu::new(tu),
        );
        weapon.insert(rebuilt);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyReloadTime, ReloadTimeScale};
    use crate::{
        magazine::{Magazine, ReloadTu},
        weapon::MagazineSize,
    };

    #[test]
    fn reload_time_scale_below_one_lowers_reload_tu() {
        let mut world = World::new();
        let weapon = world
            .spawn(Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)))
            .id();
        let mut entity = world.entity_mut(weapon);
        ApplyReloadTime::new(ReloadTimeScale::new(0.5)).apply_to_weapon(&mut entity);
        let Some(magazine) = entity.get::<Magazine>() else {
            unreachable!("the weapon must still carry a Magazine");
        };
        assert!(
            *magazine.reload_tu() < 20,
            "a < 1.0 ReloadTime scale lowers reload_tu below the 20 baseline (got {})",
            *magazine.reload_tu()
        );
    }

    #[test]
    fn reload_time_scale_above_one_raises_reload_tu() {
        let mut world = World::new();
        let weapon = world
            .spawn(Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)))
            .id();
        let mut entity = world.entity_mut(weapon);
        ApplyReloadTime::new(ReloadTimeScale::new(1.5)).apply_to_weapon(&mut entity);
        let Some(magazine) = entity.get::<Magazine>() else {
            unreachable!("the weapon must still carry a Magazine");
        };
        assert!(
            *magazine.reload_tu() > 20,
            "a > 1.0 ReloadTime scale raises reload_tu above the 20 baseline (got {})",
            *magazine.reload_tu()
        );
    }
}
