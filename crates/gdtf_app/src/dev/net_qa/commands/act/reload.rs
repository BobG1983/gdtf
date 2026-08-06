use bevy::prelude::*;
use gdtf_battle_input::ActIntent;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, NoArgs, settle_acts},
};
use crate::dev::net_qa::{
    commands::read::availability::running_and_caught, facts::GameFacts, wire::act::ActReply,
};

pub(crate) struct ActReload;

impl QaCommand for ActReload {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("act.reload");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Reload the selected shooter's weapon, taking the same path the reload keybind does. The \
         reply brackets the act log with the head before and after the frame; a reload the sim \
         declines logs nothing, so `from_seq` comes back equal to `to_seq`.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_reload.in_set(ActCommandSystems::Claim),
                settle_act_reload.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_reload(
    mut queue: ResMut<PendingQueue<CommandCall<ActReload>>>,
    mut deferred: ResMut<DeferredReplies<ActReload>>,
    mut claim: ActClaim,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    for (_args, responder) in take_calls::<ActReload>(&mut queue) {
        let Some(actor) = claim.shooter() else {
            responder.answer(&NO_SHOOTER);
            continue;
        };
        claim.push(ActIntent::Reload);
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_reload(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActReload>>) {
    settle_acts::<ActReload>(&settle, &mut deferred);
}
