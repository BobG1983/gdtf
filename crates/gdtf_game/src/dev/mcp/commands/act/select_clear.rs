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

pub(crate) struct ActSelectClear;

impl McpCommand for ActSelectClear {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = SelectReply;

    const NAME: CommandName = CommandName::from_static("act.select_clear");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Drop the selection entirely, the same path the clear keybind takes. The reply says who \
         is selected the moment the clear lands, which is nobody — but the clear does not last: \
         the game hands the player someone to act with, so the next frame re-selects the first \
         living player ganger. Name the actor with `act.select` before an act rather than \
         reading anything into the empty reply.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_select_clear.in_set(ActCommandSystems::Claim),
                settle_act_select_clear.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_select_clear(
    mut queue: ResMut<PendingQueue<CommandCall<ActSelectClear>>>,
    mut deferred: ResMut<DeferredReplies<ActSelectClear>>,
    mut pending: ResMut<PendingActIntent>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ActSelectClear>(&mut queue) {
        pending.push(ActIntent::SelectionClear);
        deferred.park(responder, ());
    }
}

fn settle_act_select_clear(
    selected: Option<Res<SelectedShooter>>,
    mut deferred: ResMut<DeferredReplies<ActSelectClear>>,
) {
    settle_selects::<ActSelectClear>(selected.as_deref(), &mut deferred);
}
