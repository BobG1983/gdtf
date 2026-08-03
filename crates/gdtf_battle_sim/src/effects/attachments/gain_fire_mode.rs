use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::{FireMode, FireModeSpec};

pub struct ApplyGainFireMode {
        mode: FireModeSpec,
}

impl ApplyGainFireMode {
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
