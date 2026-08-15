//! `editor.new` — blank the named mode's draft through that form's own constructor.

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

use super::{
    families::{BlankResult, blank_draft},
    refusal::refusal_for,
};
use crate::net_qa::{
    commands::write::availability::only_while_editing,
    facts::EditorFacts,
    forms::EditorForms,
    schedule::EditorNetQaSystems,
    wire::{EditorModeNet, EditorNewOutcomeNet},
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.new replaces a form's draft, and every draft is a resource the editor only creates \
     on entering Editing",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("that mode's draft resource is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorNewArgs {
    mode: EditorModeNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorNewReply {
    outcome: EditorNewOutcomeNet,
}

pub(in crate::net_qa) struct EditorNew;

impl QaCommand for EditorNew {
    type Args = EditorNewArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorNewReply;

    const NAME: CommandName = CommandName::from_static("editor.new");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Replace the named mode's draft with that form's own blank-draft constructor. Built for \
         the seven modes whose form draws a New button; Terrain, Prefab and Theme answer a typed \
         refusal saying why.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_DRAFTS)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_new
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_new(
    mut forms: EditorForms,
    mut queue: ResMut<PendingQueue<CommandCall<EditorNew>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<EditorNew>(&mut queue) {
        let mode = args.mode.to_mode();
        if let Some(refusal) = refusal_for(mode) {
            responder.answer(&EditorNewReply {
                outcome: EditorNewOutcomeNet::Refused(refusal),
            });
            continue;
        }
        match blank_draft(mode, &mut forms) {
            BlankResult::Blanked => responder.answer(&EditorNewReply {
                outcome: EditorNewOutcomeNet::Blanked,
            }),
            BlankResult::NoDraft => {
                responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
            }
        }
    }
}
