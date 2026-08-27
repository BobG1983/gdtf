//! `editor.weighting` — the Injury tab's weighting table as its draft holds it.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::Deserialize;

use crate::{
    injury_form::WeightingDraft,
    net_qa::{
        commands::availability::only_on_the_injury_tab, facts::EditorFacts,
        schedule::EditorNetQaSystems, wire::WeightingTableNet,
    },
};

const NO_WEIGHTING_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.weighting reads the weighting draft, which the editor only creates on entering \
     Editing",
);

const NOT_THE_INJURY_TAB: RefusalNote = RefusalNote::from_static(
    "the weighting table is drawn only in the Injury form, so this read needs the Injury tab open",
);

const DRAFT_GONE: RefusalNote = RefusalNote::from_static(
    "the weighting draft resource left the world between the availability check and the handler",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorWeightingArgs {}

pub(in crate::net_qa) struct EditorWeighting;

impl QaCommand for EditorWeighting {
    type Args = EditorWeightingArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = WeightingTableNet;

    const NAME: CommandName = CommandName::from_static("editor.weighting");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the weighting table the Injury tab holds: its category and damage context, and \
         each of its Minor, Major and Critical buckets in the order the draft holds them. The \
         keys an author may pick come from editor.families.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_injury_tab(*facts, NO_WEIGHTING_DRAFT, NOT_THE_INJURY_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_weighting
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_weighting(
    draft: Option<Res<WeightingDraft>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorWeighting>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(draft) = draft else {
        for (_args, responder) in take_calls::<EditorWeighting>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, DRAFT_GONE);
        }
        return;
    };
    for (_args, responder) in take_calls::<EditorWeighting>(&mut queue) {
        responder.answer(&WeightingTableNet::from_weighting(draft.weighting()));
    }
}
