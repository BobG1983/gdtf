use bevy::prelude::*;
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::acts::SetFacingRequested;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::Deserialize;

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, settle_acts},
};
use crate::dev::net_qa::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{act::ActReply, act_payload::FacingNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActSetFacingArgs {
    /// Compass direction to face.
    facing: FacingNet,
}

pub(crate) struct ActSetFacing;

impl QaCommand for ActSetFacing {
    type Args = ActSetFacingArgs;
    type Facts = GameFacts;
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("act.set_facing");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Turn the selected shooter to a named compass direction, the same absolute path a right \
         click takes, rather than cycling the way the keybind does. The reply brackets the act \
         log with the head before and after the frame.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_set_facing.in_set(ActCommandSystems::Claim),
                settle_act_set_facing.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_set_facing(
    mut queue: ResMut<PendingQueue<CommandCall<ActSetFacing>>>,
    mut deferred: ResMut<DeferredReplies<ActSetFacing>>,
    mut claim: ActClaim,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    for (args, responder) in take_calls::<ActSetFacing>(&mut queue) {
        let Some(actor) = claim.shooter() else {
            responder.answer(&NO_SHOOTER);
            continue;
        };
        claim.push(ActIntent::Turn(SetFacingRequested::new(
            actor,
            args.facing.to_sim(),
        )));
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_set_facing(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActSetFacing>>) {
    settle_acts::<ActSetFacing>(&settle, &mut deferred);
}
