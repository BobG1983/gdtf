use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, SelectedFireMode, ShooterArms, try_fire_request};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::Deserialize;

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, settle_acts},
};
use crate::dev::net_qa::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{act::ActReply, cell::CellLevelNet},
};

/// Why a shot answers nothing while the fire mode it would use is not loaded.
const NO_FIRE_MODE: RefusalNote =
    RefusalNote::from_static("the fire mode this shot would use is not loaded: SelectedFireMode");

/// Why a shot answers nothing while the tuning it would be costed against is not loaded.
const NO_COMBAT_TUNING: RefusalNote = RefusalNote::from_static(
    "the tuning this shot would be costed against is not loaded: CombatTuning",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActFireArgs {
    /// Cell-level to shoot at.
    at: CellLevelNet,
}

pub(crate) struct ActFire;

impl QaCommand for ActFire {
    type Args = ActFireArgs;
    type Facts = GameFacts;
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("act.fire");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Fire the selected shooter's live fire mode at a cell, building the request the pointer \
         builds. The mode is whatever battle.selection reports; this command does not change it. \
         A shot the sim declines is never declared, so the reply comes back with `from_seq` equal \
         to `to_seq` and the shooter logging nothing in that window. A host that holds no \
         SelectedFireMode or no CombatTuning cannot build the shot at all, and refuses \
         MissingModel naming the absent one rather than answering an empty window a caller would \
         read as a declined shot.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_fire.in_set(ActCommandSystems::Claim),
                settle_act_fire.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_fire(
    mut queue: ResMut<PendingQueue<CommandCall<ActFire>>>,
    mut deferred: ResMut<DeferredReplies<ActFire>>,
    mut claim: ActClaim,
    mode: Option<Res<SelectedFireMode>>,
    tuning: Option<Res<CombatTuning>>,
    arms: ShooterArms,
) {
    if queue.is_empty() {
        return;
    }
    let from_seq = claim.head();
    for (args, responder) in take_calls::<ActFire>(&mut queue) {
        let Some(mode) = mode.as_deref() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_FIRE_MODE);
            continue;
        };
        let Some(tuning) = tuning.as_deref() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_COMBAT_TUNING);
            continue;
        };
        let Some(actor) = claim.shooter() else {
            responder.answer(&NO_SHOOTER);
            continue;
        };
        if let Some(request) = try_fire_request(actor, args.at.to_sim(), mode, tuning, &arms) {
            claim.push(ActIntent::Fire(request));
        }
        deferred.park(responder, ActTicket::new(from_seq, Some(actor)));
    }
}

fn settle_act_fire(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActFire>>) {
    settle_acts::<ActFire>(&settle, &mut deferred);
}
