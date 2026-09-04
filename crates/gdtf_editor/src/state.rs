//! High-level editor app states.

use bevy::prelude::*;

/// Load vs editing lifecycle for the map editor.
#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum EditorState {
    /// Asset / registry load in progress.
    #[default]
    Load,
    /// Authoring scene is live.
    Editing,
}
