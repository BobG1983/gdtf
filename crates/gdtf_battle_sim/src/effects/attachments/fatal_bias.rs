//! The **`FatalBias`** attachment effect (GTW-549 USER-REVIEW extra; GTW-558
use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::FatalBias;

/// USER-REVIEW extra (defensible default).
pub struct ApplyFatalBias {
        bias_bonus: FatalBias,
}

impl ApplyFatalBias {
        #[must_use]
    pub const fn new(bias_bonus: FatalBias) -> Self {
        Self { bias_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyFatalBias {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(bias) = weapon.get::<FatalBias>() else {
            return;
        };
        let raised = FatalBias::new(**bias + *self.bias_bonus);
        weapon.insert(raised);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyFatalBias};
    use crate::weapon::FatalBias;

        #[test]
    fn fatal_bias_raises_fatal_bias() {
        let mut world = World::new();
        let weapon = world.spawn(FatalBias::new(2.0)).id();
        let mut entity = world.entity_mut(weapon);
        ApplyFatalBias::new(FatalBias::new(1.5)).apply_to_weapon(&mut entity);
        let Some(bias) = entity.get::<FatalBias>() else {
            unreachable!("FatalBias present");
        };
        assert!(**bias > 2.0, "FatalBias raises fatal bias above 2.0");
    }
}
