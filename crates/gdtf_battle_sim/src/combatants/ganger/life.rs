//! Alive / downed / dead state.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// Whether the ganger can still act.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active(bool);

impl Active {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(active: bool) -> Self {
        Self(active)
    }
}

/// Combat life state.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum LifeState {
    /// Standing and able to act.
    #[default]
    Alive,
    /// Down but not dead.
    Downed,
    /// Dead.
    Dead,
}

impl LifeState {
    /// True only when [`Alive`](Self::Alive).
    #[must_use]
    pub const fn is_active(self) -> Active {
        Active::new(matches!(self, Self::Alive))
    }
}
