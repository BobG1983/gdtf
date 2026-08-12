//! Input system sets.

use bevy::prelude::*;

/// System set for gathering player input before sim runs.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSystems {
    /// Collect keys, clicks, and intents.
    Gather,
}

/// The order the writers of a weapon's chosen fire mode run in.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FireModeSystems {
    /// Every write of a weapon's chosen fire mode. A reader orders itself after this.
    Write,
    /// The mode panel writing the segment the player pressed.
    Panel,
    /// A QA command writing the mode it was asked for.
    Command,
}
