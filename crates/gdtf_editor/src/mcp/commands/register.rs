//! Put the QA command sets inside the editor's own gather set and register the commands.

use bevy::prelude::*;
use cobalt_mcp_host::dispatch::{McpCommandSystems, register_command_set};

use super::set::EDITOR_COMMANDS;
use crate::mcp::schedule::EditorMcpSystems;

pub(in crate::mcp) fn register_editor_commands(app: &mut App) {
    app.configure_sets(
        Update,
        (McpCommandSystems::Route, McpCommandSystems::Claim).in_set(EditorMcpSystems::Gather),
    );
    register_command_set(app, EDITOR_COMMANDS);
}
