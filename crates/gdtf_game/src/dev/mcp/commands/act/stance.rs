use bevy::{ecs::system::SystemParam, prelude::*};
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::{
    ganger::{Stance, StanceKind, Tu},
    posture::{StanceRefusal, stance_refusal},
    tuning::CombatTuning,
};
use serde::Deserialize;

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, ActTicket, NO_SHOOTER, settle_acts},
};
use crate::dev::mcp::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{act::ActReply, act_payload::StanceNet, refusal::StanceRefusalNet},
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
         does, so the same call twice leaves the same stance. A change the sim would not make \
         answers StanceRefused naming AlreadyHeld or Unaffordable; one it would make brackets the \
         act log with the head before and after the frame.",
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

/// The pose and pool the sim's own stance guard is asked about.
#[derive(SystemParam)]
struct StanceGuard<'w, 's> {
    posers: Query<'w, 's, (&'static Stance, &'static Tu)>,
    tuning: Option<Res<'w, CombatTuning>>,
}

impl StanceGuard<'_, '_> {
    /// The sim's own reason for turning this change down, when it has one to give.
    fn refusal(&self, actor: Entity, to: StanceKind) -> Option<StanceRefusal> {
        let (stance, tu) = self.posers.get(actor).ok()?;
        let tuning = self.tuning.as_deref()?;
        stance_refusal(stance, to, tu, &tuning.stance_change_tu)
    }
}

fn claim_act_set_stance(
    mut queue: ResMut<PendingQueue<CommandCall<ActSetStance>>>,
    mut deferred: ResMut<DeferredReplies<ActSetStance>>,
    mut claim: ActClaim,
    guard: StanceGuard,
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
        let stance = args.stance.to_sim();
        if let Some(refusal) = guard.refusal(actor, stance) {
            responder.answer(&ActReply::StanceRefused {
                reason: StanceRefusalNet::from_sim(refusal),
            });
            continue;
        }
        claim.push(ActIntent::SetStance(stance));
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_set_stance(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActSetStance>>) {
    settle_acts::<ActSetStance>(&settle, &mut deferred);
}
