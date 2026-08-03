//! Straight vs arcing projectile path.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// How the projectile travels.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TrajectoryStyle {
    /// Line of sight path.
    #[default]
    Straight,
    /// Lobbed / arcing path.
    Arc,
}

/// True when trajectory is an arc.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lobbed(bool);

impl TrajectoryStyle {
    /// Whether this is an arcing shot.
    #[must_use]
    pub const fn is_arc(self) -> Lobbed {
        Lobbed(matches!(self, Self::Arc))
    }
}
