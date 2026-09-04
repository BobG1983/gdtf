use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::acts::AimRequest;
use serde::Deserialize;

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, settle_acts},
};
use crate::dev::mcp::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{act::ActReply, act_payload::AimNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActSetAimingArgs {
    /// Whether the shooter should be aiming.
    aim: AimNet,
}

pub(crate) struct ActSetAiming;

impl McpCommand for ActSetAiming {
    type Args = ActSetAimingArgs;
    type Facts = GameFacts;
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("act.set_aiming");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Set whether the selected shooter is aiming, rather than toggling the way the keybind \
         does, so the same call twice leaves the same state. The reply brackets the act log with \
         the head before and after the frame.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_set_aiming.in_set(ActCommandSystems::Claim),
                settle_act_set_aiming.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_set_aiming(
    mut queue: ResMut<PendingQueue<CommandCall<ActSetAiming>>>,
    mut deferred: ResMut<DeferredReplies<ActSetAiming>>,
    mut claim: ActClaim,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    for (args, responder) in take_calls::<ActSetAiming>(&mut queue) {
        let Some(actor) = claim.shooter() else {
            responder.answer(&NO_SHOOTER);
            continue;
        };
        claim.push(ActIntent::SetAiming(AimRequest::new(*args.aim)));
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_set_aiming(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActSetAiming>>) {
    settle_acts::<ActSetAiming>(&settle, &mut deferred);
}
