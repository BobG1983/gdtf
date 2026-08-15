//! `editor.load` — fill the named mode's draft from that family's registry.

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
    dispatch::{LoadAttempt, load_for_mode},
    families::KeyLookup,
};
use crate::net_qa::{
    commands::write::availability::only_while_editing,
    facts::EditorFacts,
    forms::{EditorForms, EditorRegistries},
    schedule::EditorNetQaSystems,
    wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.load fills a form's draft, and every draft is a resource the editor only creates on \
     entering Editing",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("that mode's draft or registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorLoadArgs {
    mode: EditorModeNet,
    key:  EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorLoadReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::net_qa) struct EditorLoad;

impl QaCommand for EditorLoad {
    type Args = EditorLoadArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadReply;

    const NAME: CommandName = CommandName::from_static("editor.load");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a registry entry by key into the named mode's draft, through that draft's own \
         `load_*` method. A key the registry does not hold answers `NoSuchKey` with the keys it \
         does hold, and leaves the draft alone.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_DRAFTS)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_load(
    mut forms: EditorForms,
    registries: EditorRegistries,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoad>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<EditorLoad>(&mut queue) {
        let mode = args.mode.to_mode();
        match load_for_mode(mode, &args.key, &mut forms, &registries) {
            LoadAttempt::Answered(KeyLookup::Loaded) => responder.answer(&EditorLoadReply {
                outcome: EditorLoadOutcomeNet::Loaded { key: args.key },
            }),
            LoadAttempt::Answered(KeyLookup::NoSuchKey(known)) => {
                responder.answer(&EditorLoadReply {
                    outcome: EditorLoadOutcomeNet::NoSuchKey {
                        key: args.key,
                        known,
                    },
                });
            }
            LoadAttempt::Refused(refusal) => responder.answer(&EditorLoadReply {
                outcome: EditorLoadOutcomeNet::Refused(refusal),
            }),
            LoadAttempt::Missing => {
                responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
            }
        }
    }
}
