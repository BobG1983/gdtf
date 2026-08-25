//! The `editor.draft` command itself: the active mode's draft, projected to RON.

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
    drafts::FormDrafts,
    project::{DraftProjection, project},
};
use crate::{
    EditorMode,
    net_qa::{
        commands::availability::only_in_a_form_mode,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{EditorDraftOutcomeNet, EditorModeNet},
    },
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.draft reads a form's draft, and every draft is a resource the editor only creates on \
     entering Editing",
);

const PREFAB_HAS_NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "the Prefab tab holds no draft to project. Read its map and session with editor.map",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("that mode's draft resource is not in the world");

const NO_MODE_RESOURCE: RefusalNote = RefusalNote::from_static(
    "the mode tab resource left the world between the availability check and the handler",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorDraftArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorDraftReply {
    mode:    EditorModeNet,
    outcome: EditorDraftOutcomeNet,
}

pub(in crate::net_qa) struct EditorDraft;

impl QaCommand for EditorDraft {
    type Args = EditorDraftArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorDraftReply;

    const NAME: CommandName = CommandName::from_static("editor.draft");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the open form's draft as the RON text its save would write, built by that form's \
         own conversion and the same serializer the writer uses. Reads the active tab and takes \
         no mode argument; the Prefab tab holds no draft and is refused.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_in_a_form_mode(*facts, NO_DRAFTS, PREFAB_HAS_NO_DRAFT)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_draft
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_draft(
    mode: Option<Res<EditorMode>>,
    drafts: FormDrafts,
    mut queue: ResMut<PendingQueue<CommandCall<EditorDraft>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mode) = mode.as_deref().copied() else {
        for (_args, responder) in take_calls::<EditorDraft>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, NO_MODE_RESOURCE);
        }
        return;
    };
    match project(mode, &drafts) {
        DraftProjection::Made(outcome) => {
            for (_args, responder) in take_calls::<EditorDraft>(&mut queue) {
                responder.answer(&EditorDraftReply {
                    mode:    EditorModeNet::from_mode(mode),
                    outcome: outcome.clone(),
                });
            }
        }
        DraftProjection::Missing => {
            for (_args, responder) in take_calls::<EditorDraft>(&mut queue) {
                responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
            }
        }
        DraftProjection::NoDraft => {
            for (_args, responder) in take_calls::<EditorDraft>(&mut queue) {
                responder.unavailable(UnavailableCode::WrongState, PREFAB_HAS_NO_DRAFT);
            }
        }
    }
}
