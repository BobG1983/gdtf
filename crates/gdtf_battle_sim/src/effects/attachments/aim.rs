//! The **Aim** attachment effect (GTW-549 HEADLINE FIX; GTW-558 one-file-per-effect) — its
//! per-item [`AimDelta`] magnitude, the isolated [`ApplyAim`] behaviour, and the `impl` that
//! raises the weapon's [`Accuracy`](crate::weapon::Accuracy).

use bevy::prelude::{Deref, EntityWorldMut};
use serde::Deserialize;

use super::ApplyAttachmentEffect;
use crate::weapon::Accuracy;

/// A sight's **aim delta** — the per-item [`Accuracy`](crate::weapon::Accuracy) addend an
/// [`Aim`](super::AttachmentEffect::Aim) attachment adds to the weapon's in-cone
/// concentration exponent (GTW-549). A precision optic RAISES accuracy (clustering the
/// §1b draw toward centre), the lever DISTINCT from stability.
///
/// A per-item authoring magnitude (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar — `Aim(0.4)`). NOT a
/// `Component` — it is an effect payload the application path reads to mutate the
/// weapon's [`Accuracy`](crate::weapon::Accuracy). Its magnitude lives HERE, on the
/// attachment item, never in global tuning (the GTW-549 headline fix).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimDelta(f32);

impl AimDelta {
    /// Build an aim delta from its [`Accuracy`](crate::weapon::Accuracy)-addend magnitude
    /// (dimensionless; a positive value tightens the in-cone draw toward centre).
    #[must_use]
    pub const fn new(delta: f32) -> Self {
        Self(delta)
    }
}

/// **Aim** — raises the weapon's [`Accuracy`](crate::weapon::Accuracy) exponent by the
/// per-item [`AimDelta`] (GTW-549 HEADLINE FIX: a sight boosts AIM — clustering the §1b
/// in-cone draw toward centre — NOT stability, which would narrow the cone).
///
/// Additive: reads the existing [`Accuracy`](crate::weapon::Accuracy) and re-inserts it
/// raised by the delta. A weapon with no `Accuracy` (a mis-seeded entity) is left
/// unchanged.
pub struct ApplyAim {
    /// The [`Accuracy`](crate::weapon::Accuracy) addend this sight contributes.
    delta: AimDelta,
}

impl ApplyAim {
    /// Build the aim effect from its per-item [`AimDelta`].
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

    /// `ApplyAim` raises the weapon's `Accuracy` above its baseline (the HEADLINE fix — a
    /// sight boosts AIM). Asserts the mapping + direction against a distinctive baseline,
    /// never a shipped magnitude.
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

    /// `ApplyAim` on a weapon with no `Accuracy` is a fail-safe no-op — it reads-then-reinserts,
    /// so it inserts nothing (no panic).
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
