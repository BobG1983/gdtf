//! `editor.set_grid_size`. Write the prefab grid extent the way the size fields commit it.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    canvas::CurrentEditLevel,
    egui_shell::prefab::size_fields::SizeFieldSpans,
    net_qa::{
        commands::availability::only_on_the_prefab_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{
            EditorGridHeightNet, EditorGridLevelsNet, EditorGridSizeNet, EditorGridWidthNet,
            EditorLevelNet,
        },
    },
    right_panel::GridSpanInput,
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.set_grid_size writes the authoring session and the edit storey, which the editor \
     only creates on entering Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the grid size fields belong to the Prefab form, so this write needs the Prefab tab open",
);

const CANVAS_GONE: RefusalNote =
    RefusalNote::from_static("the authoring session or the edit storey is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorSetGridSizeArgs {
    width:  EditorGridWidthNet,
    height: EditorGridHeightNet,
    levels: EditorGridLevelsNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorSetGridSizeReply {
    grid_size: EditorGridSizeNet,
    level:     EditorLevelNet,
}

pub(in crate::net_qa) struct EditorSetGridSize;

impl QaCommand for EditorSetGridSize {
    type Args = EditorSetGridSizeArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSetGridSizeReply;

    const NAME: CommandName = CommandName::from_static("editor.set_grid_size");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Set the prefab grid's width, height and storey count through the same commit the size \
         fields call. A span outside the band the fields allow clamps into it rather than \
         failing, and a shrink re-clamps the storey being edited. The reply echoes what was \
         written, so a caller can see a clamp happened. Needs the Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_prefab_tab(*facts, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_set_grid_size
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// The same three spans the size fields hand to their own commit.
fn spans(args: &EditorSetGridSizeArgs) -> SizeFieldSpans {
    SizeFieldSpans::new(
        GridSpanInput::new(*args.width),
        GridSpanInput::new(*args.height),
        GridSpanInput::new(*args.levels),
    )
}

fn handle_editor_set_grid_size(
    session: Option<ResMut<MapEditorSession>>,
    edit_level: Option<ResMut<CurrentEditLevel>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSetGridSize>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut session), Some(mut edit_level)) = (session, edit_level) else {
        for (_args, responder) in take_calls::<EditorSetGridSize>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, CANVAS_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSetGridSize>(&mut queue) {
        spans(&args).commit(&mut session, &mut edit_level);
        responder.answer(&EditorSetGridSizeReply {
            grid_size: EditorGridSizeNet::from_size(session.grid_size()),
            level:     EditorLevelNet::new(*edit_level.level()),
        });
    }
}
