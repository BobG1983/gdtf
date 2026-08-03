use bevy::prelude::{Component, Deref};

use crate::cover::HeightBand;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OpenState {
            #[default]
    Closed,
            Open,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DoorOpen(bool);

impl DoorOpen {
        #[must_use]
    pub const fn new(open: bool) -> Self {
        Self(open)
    }
}

impl OpenState {
        #[must_use]
    pub const fn is_open(self) -> DoorOpen {
        DoorOpen::new(matches!(self, Self::Open))
    }

            #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Closed => Self::Open,
            Self::Open => Self::Closed,
        }
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenableBlocking(HeightBand);

impl OpenableBlocking {
                #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}
