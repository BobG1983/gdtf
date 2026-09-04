//! `editor.select_facing`. Pick the side a paint turns its piece to, as the palette does.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use serde::{Deserialize, Serialize};

use crate::{
    mcp::{
        commands::availability::only_on_the_prefab_tab, facts::EditorFacts,
        schedule::EditorMcpSystems, wire::TerrainFacingNet,
    },
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.select_facing writes the authoring session, which the editor only creates on \
     entering Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the facing row belongs to the Prefab form's palette, so this write needs the Prefab tab open",
);

const SESSION_GONE: RefusalNote =
    RefusalNote::from_static("the authoring session is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSelectFacingArgs {
    facing: TerrainFacingNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSelectFacingReply {
    facing: TerrainFacingNet,
}

pub(in crate::mcp) struct EditorSelectFacing;

impl McpCommand for EditorSelectFacing {
    type Args = EditorSelectFacingArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSelectFacingReply;

    const NAME: CommandName = CommandName::from_static("editor.select_facing");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select the side the canvas turns the tile it paints to, the same choice the palette's \
         facing row writes. Both paint entry points read it off the session, so a later \
         editor.paint lands its piece on this facing. The reply names the facing the session \
         now holds. Needs the Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_prefab_tab(*facts, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_select_facing
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_select_facing(
    session: Option<ResMut<MapEditorSession>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSelectFacing>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mut session) = session else {
        for (_args, responder) in take_calls::<EditorSelectFacing>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, SESSION_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSelectFacing>(&mut queue) {
        session.set_facing(args.facing.to_facing());
        responder.answer(&EditorSelectFacingReply {
            facing: TerrainFacingNet::from_facing(session.facing()),
        });
    }
}
