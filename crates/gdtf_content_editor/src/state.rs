//! [`Editing`](EditorState::Editing) where the authoring scene lives.

use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum EditorState {
                #[default]
    Load,
        Editing,
}
