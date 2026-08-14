//! `editor.phase` — which phase the editor is in and which mode tab is open.

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
        facts::{EditorFacts, EditorFactsParam},
        schedule::EditorNetQaSystems,
        wire::{EditorModeNet, EditorPhaseNet},
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorPhaseArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorPhaseReply {
    phase: EditorPhaseNet,
    mode:  Option<EditorModeNet>,
    modes: Vec<EditorModeNet>,
}

pub(in crate::net_qa) struct EditorPhase;

impl QaCommand for EditorPhase {
    type Args = EditorPhaseArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorPhaseReply;

    const NAME: CommandName = CommandName::from_static("editor.phase");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read where the editor is — the lifecycle phase, the mode tab it has open, and every \
         tab it offers in tab-bar order. Needs no loaded content, so it answers from the moment \
         the process boots; the open tab is absent until the authoring scene is live.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &EditorFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_phase
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_phase(
    facts: EditorFactsParam,
    mut queue: ResMut<PendingQueue<CommandCall<EditorPhase>>>,
) {
    if queue.is_empty() {
        return;
    }
    let sampled = facts.sample();
    let modes = EditorMode::TAB_ORDER.map(EditorModeNet::from_mode);
    for (_args, responder) in take_calls::<EditorPhase>(&mut queue) {
        responder.answer(&EditorPhaseReply {
            phase: sampled.phase(),
            mode:  sampled.mode(),
            modes: modes.to_vec(),
        });
    }
}
