//! The [`OnDeath`] authoring component — the weapon-entity carrier of an authored
use bevy::prelude::{Component, Deref};

use crate::effects::{fields::FieldKey, on_death::OnDeathEffect};

/// The **authoring component** carrying a weapon / gear entity's [`OnDeathEffect`]
#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct OnDeath(OnDeathEffect);

impl OnDeath {
        #[must_use]
    pub const fn new(effect: OnDeathEffect) -> Self {
        Self(effect)
    }

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
