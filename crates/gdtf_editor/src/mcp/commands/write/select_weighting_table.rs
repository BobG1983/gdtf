//! `editor.select_weighting_table` — load one category and context table into the draft.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::injuries::InjuryTables;
use serde::Deserialize;

use crate::{
    injury_form::WeightingDraft,
    mcp::{
        commands::availability::only_on_the_injury_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{DamageContextNet, InjuryCategoryNet, WeightingTableNet},
    },
};

const NO_WEIGHTING_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.select_weighting_table writes the weighting draft, which the editor only creates on \
     entering Editing",
);

const NOT_THE_INJURY_TAB: RefusalNote = RefusalNote::from_static(
    "the weighting table selectors belong to the Injury form, so this write needs the Injury tab \
     open",
);

const DRAFT_GONE: RefusalNote = RefusalNote::from_static(
    "the weighting draft resource left the world between the availability check and the handler",
);

const NO_TABLES: RefusalNote = RefusalNote::from_static(
    "the injury tables are not in the world, and the form's own selectors draw `(loading…)` and \
     touch nothing while that is true",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSelectWeightingTableArgs {
    category: InjuryCategoryNet,
    context:  DamageContextNet,
}

pub(in crate::mcp) struct EditorSelectWeightingTable;

impl McpCommand for EditorSelectWeightingTable {
    type Args = EditorSelectWeightingTableArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = WeightingTableNet;

    const NAME: CommandName = CommandName::from_static("editor.select_weighting_table");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load the weighting table for one injury category and damage context, the way the \
         Injury tab's two selectors load it. All three buckets are replaced from the tables, so \
         unsaved row edits are discarded.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_injury_tab(*facts, NO_WEIGHTING_DRAFT, NOT_THE_INJURY_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_select_weighting_table
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_select_weighting_table(
    draft: Option<ResMut<WeightingDraft>>,
    tables: Option<Res<InjuryTables>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSelectWeightingTable>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mut draft) = draft else {
        for (_args, responder) in take_calls::<EditorSelectWeightingTable>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, DRAFT_GONE);
        }
        return;
    };
    let Some(tables) = tables else {
        for (_args, responder) in take_calls::<EditorSelectWeightingTable>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, NO_TABLES);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSelectWeightingTable>(&mut queue) {
        draft.load_table(
            args.category.to_category(),
            args.context.to_context(),
            &tables,
        );
        responder.answer(&WeightingTableNet::from_weighting(draft.weighting()));
    }
}
