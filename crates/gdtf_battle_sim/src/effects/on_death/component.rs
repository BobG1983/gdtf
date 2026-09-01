//! On-death component carried by weapon entities.

use bevy::prelude::{Component, Deref};

use crate::effects::on_death::OnDeathEffect;

/// Weapon/gear carrier for the authored on-death effects.
#[derive(Component, Deref, Debug, Clone, Default, PartialEq)]
pub struct OnDeath(Vec<OnDeathEffect>);

impl OnDeath {
    /// Wrap a list of effects.
    #[must_use]
    pub const fn new(effects: Vec<OnDeathEffect>) -> Self {
        Self(effects)
    }

    /// Inner effects, in the authored order.
    #[must_use]
    pub fn effects(&self) -> &[OnDeathEffect] {
        &self.0
    }
}
