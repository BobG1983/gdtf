//! On-death component carried by weapon entities.

use bevy::prelude::{Component, Deref};

use crate::effects::{fields::FieldKey, on_death::OnDeathEffect};

/// Weapon/gear carrier for an authored on-death effect.
#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct OnDeath(OnDeathEffect);

impl OnDeath {
    /// Wrap an effect.
    #[must_use]
    pub const fn new(effect: OnDeathEffect) -> Self {
        Self(effect)
    }

    /// Inner effect.
    #[must_use]
    pub const fn effect(&self) -> &OnDeathEffect {
        &self.0
    }
}

impl Default for OnDeath {
    fn default() -> Self {
        Self(OnDeathEffect::LeaveField {
            field: FieldKey::new(String::new()),
        })
    }
}
