//! `editor.save_weighting` — write the weighting draft under the QA assets root.

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
    EditorMode, injury_form,
    injury_form::WeightingDraft,
    mcp::{
        assets_root::EditorMcpAssetsRoot, commands::availability::only_on_the_injury_tab,
        facts::EditorFacts, schedule::EditorMcpSystems, wire::EditorSaveOutcomeNet,
    },
    save_record::{LastSaveRecord, SaveOutcome},
};

const NO_WEIGHTING_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.save_weighting writes the weighting draft, which the editor only creates on entering \
     Editing",
);

const NOT_THE_INJURY_TAB: RefusalNote = RefusalNote::from_static(
    "the Save weighting button belongs to the Injury form, so this write needs the Injury tab \
     open",
);

const DRAFT_GONE: RefusalNote = RefusalNote::from_static(
    "the weighting draft resource left the world between the availability check and the handler",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSaveWeightingArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSaveWeightingReply {
    outcome: EditorSaveOutcomeNet,
}

pub(in crate::mcp) struct EditorSaveWeighting;

impl McpCommand for EditorSaveWeighting {
    type Args = EditorSaveWeightingArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSaveWeightingReply;

    const NAME: CommandName = CommandName::from_static("editor.save_weighting");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Write the Injury weighting draft through the same writer its Save weighting button \
         calls, under the QA assets root. A writer that fails answers a Failed outcome inside a \
         Ran reply, and records it the way the button does.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_injury_tab(*facts, NO_WEIGHTING_DRAFT, NOT_THE_INJURY_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_save_weighting
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_save_weighting(
    draft: Option<Res<WeightingDraft>>,
    root: Res<EditorMcpAssetsRoot>,
    mut record: ResMut<LastSaveRecord>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSaveWeighting>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(draft) = draft else {
        for (_args, responder) in take_calls::<EditorSaveWeighting>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, DRAFT_GONE);
        }
        return;
    };
    for (_args, responder) in take_calls::<EditorSaveWeighting>(&mut queue) {
        let weighting = injury_form::draft_to_weighting(&draft);
        let outcome = SaveOutcome::from_result(injury_form::write_weighting_in(&root, &weighting));
        let wire = EditorSaveOutcomeNet::from_outcome(&outcome);
        record.record(EditorMode::Injury, outcome);
        responder.answer(&EditorSaveWeightingReply { outcome: wire });
    }
}
