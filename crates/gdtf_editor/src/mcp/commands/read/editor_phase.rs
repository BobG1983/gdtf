//! `editor.phase` — which phase the editor is in and which mode tab is open.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use serde::{Deserialize, Serialize};

use crate::{
    EditorMode,
    mcp::{
        facts::{EditorFacts, EditorFactsParam},
        schedule::EditorMcpSystems,
        wire::{EditorModeNet, EditorPhaseNet},
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorPhaseArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorPhaseReply {
    phase: EditorPhaseNet,
    mode:  Option<EditorModeNet>,
    modes: Vec<EditorModeNet>,
}

pub(in crate::mcp) struct EditorPhase;

impl McpCommand for EditorPhase {
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
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
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
