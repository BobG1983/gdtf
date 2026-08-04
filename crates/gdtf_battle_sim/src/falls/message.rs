//! Messages emitted when a fall completes.

use bevy::prelude::{Entity, Message};

use crate::metric::Level;

/// A ganger fell from one level to another.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FallOccurred {
    /// Who fell.
    pub ganger:     Entity,
    /// Level before the fall.
    pub from_level: Level,
    /// Level after the fall.
    pub to_level:   Level,
    /// How many storeys were dropped.
    pub storeys:    StoreysFallen,
}

impl FallOccurred {
    /// Build a fall message.
    #[must_use]
    pub const fn new(
        ganger: Entity,
        from_level: Level,
        to_level: Level,
        storeys: StoreysFallen,
    ) -> Self {
        Self {
            ganger,
            from_level,
            to_level,
            storeys,
        }
    }
}

/// Number of storeys fallen.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StoreysFallen(u8);

impl StoreysFallen {
    /// Wrap a storey count.
    #[must_use]
    pub const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}
