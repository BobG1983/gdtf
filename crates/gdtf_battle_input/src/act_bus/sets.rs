//! Input system sets.

use bevy::prelude::*;

/// System set for gathering player input before sim runs.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSystems {
    /// Collect keys, clicks, and intents.
    Gather,
}
