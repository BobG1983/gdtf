use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};

use super::{
    sets::ActCommandSystems,
    support::{NoArgs, settle_selects},
};
use crate::dev::mcp::{
    commands::read::availability::running_and_caught, facts::GameFacts, wire::act::SelectReply,
};

pub(crate) struct ActSelectNext;

impl McpCommand for ActSelectNext {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = SelectReply;

    const NAME: CommandName = CommandName::from_static("act.select_next");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Step the selection to the next living player ganger in the order the cycle keybind \
         walks, wrapping at the end. The reply says who is selected once the frame has run.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_select_next.in_set(ActCommandSystems::Claim),
                settle_act_select_next.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

pub(crate) struct ActSelectPrev;

impl McpCommand for ActSelectPrev {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = SelectReply;

    const NAME: CommandName = CommandName::from_static("act.select_prev");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Step the selection to the previous living player ganger in the order the cycle keybind \
         walks, wrapping at the start. The reply says who is selected once the frame has run.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_select_prev.in_set(ActCommandSystems::Claim),
                settle_act_select_prev.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_select_next(
    mut queue: ResMut<PendingQueue<CommandCall<ActSelectNext>>>,
    mut deferred: ResMut<DeferredReplies<ActSelectNext>>,
    mut pending: ResMut<PendingActIntent>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ActSelectNext>(&mut queue) {
        pending.push(ActIntent::SelectNext);
        deferred.park(responder, ());
    }
}

fn settle_act_select_next(
    selected: Option<Res<SelectedShooter>>,
    mut deferred: ResMut<DeferredReplies<ActSelectNext>>,
) {
    settle_selects::<ActSelectNext>(selected.as_deref(), &mut deferred);
}

fn claim_act_select_prev(
    mut queue: ResMut<PendingQueue<CommandCall<ActSelectPrev>>>,
    mut deferred: ResMut<DeferredReplies<ActSelectPrev>>,
    mut pending: ResMut<PendingActIntent>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ActSelectPrev>(&mut queue) {
        pending.push(ActIntent::SelectPrev);
        deferred.park(responder, ());
    }
}

fn settle_act_select_prev(
    selected: Option<Res<SelectedShooter>>,
    mut deferred: ResMut<DeferredReplies<ActSelectPrev>>,
) {
    settle_selects::<ActSelectPrev>(selected.as_deref(), &mut deferred);
}
