//! `editor.save` — write the named mode's draft under the QA assets root.

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

use super::dispatch::{SaveAttempt, name_refusal, save_for_mode};
use crate::{
    net_qa::{
        assets_root::EditorQaAssetsRoot,
        commands::write::availability::only_while_editing,
        facts::EditorFacts,
        forms::{EditorForms, EditorRegistries},
        schedule::EditorNetQaSystems,
        wire::{
            EditorContentNameNet, EditorModeNet, EditorSaveFaultNet, EditorSaveOutcomeNet,
            SavedPathNet,
        },
    },
    save_record::{LastSaveRecord, SaveOutcome},
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.save writes a form's draft, and every draft is a resource the editor only creates on \
     entering Editing",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("that mode's draft or registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorSaveArgs {
    mode: EditorModeNet,
    #[serde(default)]
    name: Option<EditorContentNameNet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorSaveReply {
    outcome: EditorSaveOutcomeNet,
}

pub(in crate::net_qa) struct EditorSave;

impl QaCommand for EditorSave {
    type Args = EditorSaveArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSaveReply;

    const NAME: CommandName = CommandName::from_static("editor.save");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Write the named mode's draft through the same writer that mode's save button calls, \
         under the QA assets root. Only Prefab takes a name; every other mode derives it from \
         the draft and refuses one.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_DRAFTS)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_save
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn wire_outcome(outcome: &SaveOutcome) -> EditorSaveOutcomeNet {
    match outcome {
        SaveOutcome::Wrote(path) => EditorSaveOutcomeNet::Wrote {
            path: SavedPathNet::from_path(path),
        },
        SaveOutcome::Failed(fault) => {
            EditorSaveOutcomeNet::Failed(EditorSaveFaultNet::from_fault(fault))
        }
    }
}

fn handle_editor_save(
    mut forms: EditorForms,
    registries: EditorRegistries,
    root: Res<EditorQaAssetsRoot>,
    mut record: ResMut<LastSaveRecord>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSave>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<EditorSave>(&mut queue) {
        let mode = args.mode.to_mode();
        if let Some(refusal) = name_refusal(mode, args.name.as_ref()) {
            responder.answer(&EditorSaveReply {
                outcome: EditorSaveOutcomeNet::Refused(refusal),
            });
            continue;
        }
        match save_for_mode(mode, args.name.as_ref(), &root, &mut forms, &registries) {
            SaveAttempt::Wrote(outcome) => {
                let wire = wire_outcome(&outcome);
                record.record(mode, outcome);
                responder.answer(&EditorSaveReply { outcome: wire });
            }
            SaveAttempt::Missing => {
                responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
            }
        }
    }
}
