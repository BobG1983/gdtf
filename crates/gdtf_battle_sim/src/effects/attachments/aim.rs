//! Aim delta attachment effect.

use bevy::prelude::{Deref, EntityWorldMut};
use serde::{Deserialize, Serialize};

use super::ApplyAttachmentEffect;
use crate::weapon::Accuracy;

/// Additive accuracy change from an attachment.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AimDelta(f32);

impl AimDelta {
    /// Wrap a delta.
    #[must_use]
    pub const fn new(delta: f32) -> Self {
        Self(delta)
    }
}

/// Applies an aim delta to a weapon's accuracy.
pub struct ApplyAim {
    delta: AimDelta,
}

impl ApplyAim {
    /// Build the applicator.
    #[must_use]
    pub const fn new(delta: AimDelta) -> Self {
        Self { delta }
    }
}

impl ApplyAttachmentEffect for ApplyAim {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(accuracy) = weapon.get::<Accuracy>() else {
            return;
        };
        let raised = Accuracy::new(**accuracy + *self.delta);
        weapon.insert(raised);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{AimDelta, ApplyAim, ApplyAttachmentEffect};
    use crate::weapon::Accuracy;

    #[test]
    fn aim_raises_accuracy() {
        let mut world = World::new();
        let weapon = world.spawn(Accuracy::new(1.0)).id();
        let mut entity = world.entity_mut(weapon);
        ApplyAim::new(AimDelta::new(0.4)).apply_to_weapon(&mut entity);
        let Some(accuracy) = entity.get::<Accuracy>() else {
            unreachable!("the weapon must still carry Accuracy");
        };
        assert!(
            **accuracy > 1.0,
            "Aim raises Accuracy above the 1.0 baseline (got {})",
            **accuracy
        );
    }

    #[test]
    fn absent_accuracy_is_a_noop() {
        let mut world = World::new();
        let weapon = world.spawn_empty().id();
        let mut entity = world.entity_mut(weapon);
        ApplyAim::new(AimDelta::new(0.4)).apply_to_weapon(&mut entity);
        assert!(
            entity.get::<Accuracy>().is_none(),
            "Aim on a weapon with no Accuracy is a fail-safe no-op"
        );
    }
}
