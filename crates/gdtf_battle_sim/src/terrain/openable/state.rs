//! Open/closed state and blocking band for openable terrain.

use bevy::prelude::{Component, Deref};

use crate::cover::HeightBand;

/// Whether an openable is closed or open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OpenState {
    /// Blocks path and vision.
    #[default]
    Closed,
    /// Does not block.
    Open,
}

/// Whether the door is currently open.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DoorOpen(bool);

impl DoorOpen {
    /// Wrap an open flag.
    #[must_use]
    pub const fn new(open: bool) -> Self {
        Self(open)
    }
}

impl OpenState {
    /// True when Open.
    #[must_use]
    pub const fn is_open(self) -> DoorOpen {
        DoorOpen::new(matches!(self, Self::Open))
    }

    /// Flip between Closed and Open.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Closed => Self::Open,
            Self::Open => Self::Closed,
        }
    }
}

/// Height band used for vision blocking when the openable is closed.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenableBlocking(HeightBand);

impl OpenableBlocking {
    /// Wrap a height band.
    #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}
