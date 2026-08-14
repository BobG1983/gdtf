//! Put the QA command sets inside the editor's own gather set and register the commands.

use bevy::prelude::*;
use gdtf_qa_command::dispatch::{QaCommandSystems, register_command_set};

use super::set::EDITOR_COMMANDS;
use crate::net_qa::schedule::EditorNetQaSystems;

pub(in crate::net_qa) fn register_editor_commands(app: &mut App) {
    app.configure_sets(
        Update,
        (QaCommandSystems::Route, QaCommandSystems::Claim).in_set(EditorNetQaSystems::Gather),
    );
    register_command_set(app, EDITOR_COMMANDS);
}
