//! The body a ray passes through instead of stopping on.

use bevy::prelude::{Deref, Entity};

/// A body the ray ignores, such as a mover's own body while it is asked what it would see
/// from somewhere else. The ray crosses that cell as if it were empty.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkippedOccupant(Option<Entity>);

impl SkippedOccupant {
    /// A march that stops on every body it crosses.
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    /// A march that crosses this body as if the cell were empty.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// Whether this is the body the ray ignores.
    #[must_use]
    pub fn skips(self, entity: Entity) -> bool {
        self.0 == Some(entity)
    }
}
