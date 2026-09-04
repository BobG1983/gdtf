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
    acts::SetFacingRequested,
    ganger::{Direction, Facing, Tu},
    posture::{FacingRefusal, facing_refusal},
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
    wire::{act::ActReply, act_payload::FacingNet, refusal::FacingRefusalNet},
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
         click takes, rather than cycling the way the keybind does. A turn the sim would not make \
         answers FacingRefused naming AlreadyFacing or Unaffordable; one it would make brackets \
         the act log with the head before and after the frame.",
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

/// The facing and pool the sim's own turn guard is asked about.
#[derive(SystemParam)]
struct FacingGuard<'w, 's> {
    turners: Query<'w, 's, (&'static Facing, &'static Tu)>,
    tuning:  Option<Res<'w, CombatTuning>>,
}

impl FacingGuard<'_, '_> {
    /// The sim's own reason for turning this turn down, when it has one to give.
    fn refusal(&self, actor: Entity, to: Direction) -> Option<FacingRefusal> {
        let (facing, tu) = self.turners.get(actor).ok()?;
        let tuning = self.tuning.as_deref()?;
        facing_refusal(**facing, to, tu, &tuning.turn_tu)
    }
}

fn claim_act_set_facing(
    mut queue: ResMut<PendingQueue<CommandCall<ActSetFacing>>>,
    mut deferred: ResMut<DeferredReplies<ActSetFacing>>,
    mut claim: ActClaim,
    guard: FacingGuard,
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
        let facing = args.facing.to_sim();
        if let Some(refusal) = guard.refusal(actor, facing) {
            responder.answer(&ActReply::FacingRefused {
                reason: FacingRefusalNet::from_sim(refusal),
            });
            continue;
        }
        claim.push(ActIntent::Turn(SetFacingRequested::new(actor, facing)));
        deferred.park(responder, ActTicket::new(from, Some(actor)));
    }
}

fn settle_act_set_facing(settle: ActSettle, mut deferred: ResMut<DeferredReplies<ActSetFacing>>) {
    settle_acts::<ActSetFacing>(&settle, &mut deferred);
}
