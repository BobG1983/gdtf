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

use crate::dev::mcp::{
    facts::{GameFacts, GameFactsParam},
    wire::AppPhaseNet,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppPhaseArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AppPhaseReply {
    phase: AppPhaseNet,
}

pub(crate) struct AppPhase;

impl McpCommand for AppPhase {
    type Args = AppPhaseArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = AppPhaseReply;

    const NAME: CommandName = CommandName::from_static("app.phase");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read where the app is at every level of its state machine — the lifecycle phase \
         plus the running screen, game layer, battle phase and aftermath phase where each \
         is live. Needs no battle, so it answers from the moment the process boots; poll \
         it to wait for a state.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_app_phase.after(McpCommandSystems::Claim));
    }
}

fn handle_app_phase(facts: GameFactsParam, mut queue: ResMut<PendingQueue<CommandCall<AppPhase>>>) {
    if queue.is_empty() {
        return;
    }
    let sampled = facts.sample();
    for (_args, responder) in take_calls::<AppPhase>(&mut queue) {
        responder.answer(&AppPhaseReply {
            phase: sampled.phase(),
        });
    }
}
