use bevy::prelude::*;
use gdtf_battle_input::ActIntent;
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
    wire::{act::ActReply, act_payload::StanceNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActSetStanceArgs {
    /// Posture to take: Standing, Crouching or Prone.
    stance: StanceNet,
}

pub(crate) struct ActSetStance;

impl QaCommand for ActSetStance {
    type Args = ActSetStanceArgs;
    type Facts = GameFacts;
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("act.set_stance");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Put the selected shooter in a named posture, rather than cycling the way the keybind \
         does, so the same call twice leaves the same stance. The reply brackets the act log with \
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
                claim_act_set_stance.in_set(ActCommandSystems::Claim),
                settle_act_set_stance.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_set_stance(
    mut queue: ResMut<PendingQueue<CommandCall<ActSetStance>>>,
    mut deferred: ResMut<DeferredReplies<ActSetStance>>,
    mut claim: ActClaim,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    for (args, responder) in take_calls::<ActSetStance>(&mut queue) {
        let Some(actor) = claim.shooter() else {
            responder.answer(&NO_SHOOTER);
            continue;
        };
        claim.push(ActIntent::SetStance(args.stance.to_sim()));
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_set_stance(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActSetStance>>) {
    settle_acts::<ActSetStance>(&settle, &mut deferred);
}
