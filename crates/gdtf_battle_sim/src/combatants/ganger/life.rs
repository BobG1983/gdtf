use bevy::prelude::{Component, Deref};
use serde::Deserialize;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active(bool);

impl Active {
        #[must_use]
    pub const fn new(active: bool) -> Self {
        Self(active)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum LifeState {
        #[default]
    Alive,
            Downed,
        Dead,
}

impl LifeState {
                                        #[must_use]
    pub const fn is_active(self) -> Active {
        Active::new(matches!(self, Self::Alive))
    }
}
