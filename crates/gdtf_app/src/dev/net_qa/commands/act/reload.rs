use bevy::prelude::*;
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::act_log::ActDeed;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, NoArgs, settle_acts_with},
};
use crate::dev::net_qa::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{act::ActReply, refusal::ReloadRefusalNet},
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
         sim records every reload it hears, whatever came of it: a refill answers an accepted \
         window bracketing the act log, and a reload the sim declines answers ReloadRefused \
         naming AlreadyFull or NoTu.",
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
    settle_acts_with::<ActReload>(&mut deferred, |ticket| reload_reply(&settle, ticket));
}

/// The refusal the sim recorded for this reload, or the window it opened.
fn reload_reply(settle: &ActSettle, ticket: &ActTicket) -> ActReply {
    let refused = settle.deeds_of(ticket).find_map(|deed| {
        let ActDeed::Reloaded { outcome } = deed else {
            return None;
        };
        ReloadRefusalNet::from_sim(*outcome)
    });
    match refused {
        Some(reason) => ActReply::ReloadRefused { reason },
        None => settle.window_of(ticket),
    }
}
