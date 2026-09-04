use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::acts::MoveRequested;
use serde::Deserialize;

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, move_reply, settle_acts_with},
};
use crate::dev::mcp::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{act::ActReply, cell::CellLevelNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActMoveArgs {
    /// Destination cell-level, as `battle.roster` and `battle.visible` report cells.
    at: CellLevelNet,
}

pub(crate) struct ActMove;

impl McpCommand for ActMove {
    type Args = ActMoveArgs;
    type Facts = GameFacts;
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("act.move");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Walk the selected shooter to a cell, taking the same path a left click does. The reply \
         brackets the act log with the head before and after the frame, and reports complete as \
         false while the route is still being walked. The sim decides legality on its own: a \
         refused move answers MoveRefused with the sim's own reason, and is logged as MoveRefused \
         with that same reason.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_move.in_set(ActCommandSystems::Claim),
                settle_act_move.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_move(
    mut queue: ResMut<PendingQueue<CommandCall<ActMove>>>,
    mut deferred: ResMut<DeferredReplies<ActMove>>,
    mut claim: ActClaim,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    for (args, responder) in take_calls::<ActMove>(&mut queue) {
        let Some(actor) = claim.shooter() else {
            responder.answer(&NO_SHOOTER);
            continue;
        };
        claim.push(ActIntent::Move(MoveRequested::new(actor, args.at.to_sim())));
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_move(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActMove>>) {
    settle_acts_with::<ActMove>(&mut deferred, |ticket| move_reply(&settle, ticket));
}
