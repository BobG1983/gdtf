//! The one list of commands the editor host publishes.

use gdtf_qa_command::command::ErasedCommand;

use super::{
    read::{
        EditorDraft, EditorFamilies, EditorLastSave, EditorPhase, EditorSession, EditorValidation,
    },
    write::{
        EditorListOp, EditorLoad, EditorNew, EditorSave, EditorSelectTheme, EditorSetDefaultFloor,
        EditorSetField, EditorSetMode, EditorToggleTerrain,
    },
};
use crate::net_qa::facts::EditorFacts;

pub(in crate::net_qa) const EDITOR_COMMANDS: &[&dyn ErasedCommand<EditorFacts>] = &[
    &EditorPhase,
    &EditorLastSave,
    &EditorValidation,
    &EditorFamilies,
    &EditorSession,
    &EditorDraft,
    &EditorSetMode,
    &EditorNew,
    &EditorLoad,
    &EditorSave,
    &EditorSetField,
    &EditorListOp,
    &EditorSelectTheme,
    &EditorToggleTerrain,
    &EditorSetDefaultFloor,
];
