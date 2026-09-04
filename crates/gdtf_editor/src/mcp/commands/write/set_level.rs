//! `editor.set_level`. Jump the edit storey the way the level rail's rows do.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::metric::Level;
use serde::{Deserialize, Serialize};

use crate::{
    canvas::CurrentEditLevel,
    mcp::{
        commands::availability::only_on_the_prefab_tab, facts::EditorFacts,
        schedule::EditorMcpSystems, wire::EditorLevelNet,
    },
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.set_level writes the edit storey, which the editor only creates on entering Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the level rail belongs to the Prefab form, so this write needs the Prefab tab open",
);

const CANVAS_GONE: RefusalNote =
    RefusalNote::from_static("the edit storey or the authoring session is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSetLevelArgs {
    level: EditorLevelNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSetLevelReply {
    level: EditorLevelNet,
}

pub(in crate::mcp) struct EditorSetLevel;

impl QaCommand for EditorSetLevel {
    type Args = EditorSetLevelArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSetLevelReply;

    const NAME: CommandName = CommandName::from_static("editor.set_level");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Jump the storey the canvas paints on, the way clicking a row of the level rail does. A \
         storey past the grid's extent clamps to the nearest one the grid has rather than being \
         refused, and the reply echoes the storey that was reached. Needs the Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_prefab_tab(*facts, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_set_level
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_set_level(
    edit_level: Option<ResMut<CurrentEditLevel>>,
    session: Option<Res<MapEditorSession>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSetLevel>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut edit_level), Some(session)) = (edit_level, session) else {
        for (_args, responder) in take_calls::<EditorSetLevel>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, CANVAS_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSetLevel>(&mut queue) {
        *edit_level = CurrentEditLevel::jumped(Level::new(*args.level), session.grid_size());
        responder.answer(&EditorSetLevelReply {
            level: EditorLevelNet::new(*edit_level.level()),
        });
    }
}
