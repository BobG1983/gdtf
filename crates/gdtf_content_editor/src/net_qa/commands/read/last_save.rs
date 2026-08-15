//! `editor.last_save` — what the newest save per mode did, whichever path drove it.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use crate::{
    EditorMode,
    net_qa::{
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{EditorLastSaveRowNet, EditorModeNet, LastSaveOutcomeNet},
    },
    save_record::LastSaveRecord,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorLastSaveArgs {
    #[serde(default)]
    mode: Option<EditorModeNet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorLastSaveReply {
    records: Vec<EditorLastSaveRowNet>,
}

pub(in crate::net_qa) struct EditorLastSave;

impl QaCommand for EditorLastSave {
    type Args = EditorLastSaveArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLastSaveReply;

    const NAME: CommandName = CommandName::from_static("editor.last_save");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read what the newest save for each mode did — the file it wrote, or the fault it \
         reported. Records survive a Load round trip, so this answers \"nothing saved yet\" \
         rather than refusing at any point in the lifecycle.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &EditorFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_last_save
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// Tab-bar order, so a client reading every mode sees the same order every time.
fn rows(record: Option<&LastSaveRecord>, only: Option<EditorModeNet>) -> Vec<EditorLastSaveRowNet> {
    let Some(record) = record else {
        return Vec::new();
    };
    EditorMode::TAB_ORDER
        .into_iter()
        .filter(|mode| only.is_none_or(|wanted| wanted.to_mode() == *mode))
        .filter_map(|mode| {
            record.outcome(mode).map(|outcome| {
                EditorLastSaveRowNet::new(
                    EditorModeNet::from_mode(mode),
                    LastSaveOutcomeNet::from_outcome(outcome),
                )
            })
        })
        .collect()
}

fn handle_editor_last_save(
    record: Option<Res<LastSaveRecord>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLastSave>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<EditorLastSave>(&mut queue) {
        responder.answer(&EditorLastSaveReply {
            records: rows(record.as_deref(), args.mode),
        });
    }
}
