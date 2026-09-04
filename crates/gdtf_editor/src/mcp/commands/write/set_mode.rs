//! `editor.set_mode` — open a mode tab, writing the resource the tab bar writes.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    EditorMode,
    mcp::{
        commands::availability::only_while_editing, facts::EditorFacts, schedule::EditorMcpSystems,
        wire::EditorModeNet,
    },
};

const NO_TAB_BAR: RefusalNote = RefusalNote::from_static(
    "editor.set_mode writes the mode tab resource, which the editor only creates on entering \
     Editing",
);

const NO_MODE_RESOURCE: RefusalNote = RefusalNote::from_static(
    "the mode tab resource left the world between the availability check and the handler",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSetModeArgs {
    mode: EditorModeNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSetModeReply {
    mode: EditorModeNet,
}

pub(in crate::mcp) struct EditorSetMode;

impl McpCommand for EditorSetMode {
    type Args = EditorSetModeArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSetModeReply;

    const NAME: CommandName = CommandName::from_static("editor.set_mode");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Open a mode tab, writing the same `EditorMode` resource the tab bar and the number \
         hotkeys write. Needs the authoring scene, so it refuses while the editor is still \
         loading its registries.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_TAB_BAR)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_set_mode
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_set_mode(
    mode: Option<ResMut<EditorMode>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSetMode>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mut mode) = mode else {
        for (_args, responder) in take_calls::<EditorSetMode>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, NO_MODE_RESOURCE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSetMode>(&mut queue) {
        *mode = args.mode.to_mode();
        responder.answer(&EditorSetModeReply { mode: args.mode });
    }
}
