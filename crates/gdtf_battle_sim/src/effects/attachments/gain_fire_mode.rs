//! The **`GainFireMode`** attachment effect (GTW-549; GTW-558 one-file-per-effect) — the
//! isolated [`ApplyGainFireMode`] behaviour and the `impl` that appends a
//! [`FireModeSpec`](crate::weapon::FireModeSpec) to the weapon's fire-mode selector. No
//! per-item magnitude newtype — its payload is the reused weapon
//! [`FireModeSpec`](crate::weapon::FireModeSpec).

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::{FireMode, FireModeSpec};

/// **`GainFireMode`** — ADDS one [`FireModeSpec`](crate::weapon::FireModeSpec) to the
/// weapon's [`FireMode`](crate::weapon::FireMode) selector (a conversion kit granting a new
/// firing mode).
///
/// Reads the existing selector, appends the new mode, and re-inserts the rebuilt
/// [`FireMode`](crate::weapon::FireMode). A weapon with no selector is left unchanged.
pub struct ApplyGainFireMode {
    /// The fire mode this conversion kit adds to the selector.
    mode: FireModeSpec,
}

impl ApplyGainFireMode {
    /// Build the gain-fire-mode effect from the [`FireModeSpec`](crate::weapon::FireModeSpec)
    /// it grants.
    #[must_use]
    pub const fn new(mode: FireModeSpec) -> Self {
        Self { mode }
    }
}

impl ApplyAttachmentEffect for ApplyGainFireMode {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(fire_mode) = weapon.get::<FireMode>() else {
            return;
        };
        let mut modes: Vec<FireModeSpec> = fire_mode.iter().copied().collect();
        modes.push(self.mode);
        weapon.insert(FireMode::new(modes));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyGainFireMode};
    use crate::weapon::{FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent};

    /// `ApplyGainFireMode` ADDS exactly one mode to the selector — the count grows by one.
    #[test]
    fn gain_fire_mode_appends_a_mode() {
        let mut world = World::new();
        let weapon = world
            .spawn(FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.2),
                ModeShots::new(1),
            )]))
            .id();
        let before = world
            .entity(weapon)
            .get::<FireMode>()
            .map_or(0, |m| m.len());
        let burst = FireModeSpec::new(
            ModeKind::Burst,
            ModeConeMult::new(1.3),
            ModeTuPercent::new(0.5),
            ModeShots::new(3),
        );
        let mut entity = world.entity_mut(weapon);
        ApplyGainFireMode::new(burst).apply_to_weapon(&mut entity);
        let after = entity.get::<FireMode>().map_or(0, |m| m.len());
        assert_eq!(after, before + 1, "GainFireMode appends exactly one mode");
    }
}
