//! Live editor resources a command's facts are sampled from.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::EditorFacts;
use crate::{
    EditorMode, EditorState,
    net_qa::wire::{EditorModeNet, EditorPhaseNet},
};

/// The editor state machine, plus the mode tab that only exists while editing.
#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorFactsParam<'w> {
    state: Res<'w, State<EditorState>>,
    mode:  Option<Res<'w, EditorMode>>,
}

impl EditorFactsParam<'_> {
    /// Read the editor's live phase and mode into a facts value.
    #[must_use]
    pub(in crate::net_qa) fn sample(&self) -> EditorFacts {
        EditorFacts::new(
            EditorPhaseNet::from_state(self.state.get()),
            self.mode
                .as_ref()
                .map(|mode| EditorModeNet::from_mode(**mode)),
        )
    }
}
