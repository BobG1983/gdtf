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
use gdtf_battle_sim::prelude::{Faction, LifeState};
use serde::Deserialize;

use super::{
    sets::ActCommandSystems,
    support::{a_ganger, settle_selects},
};
use crate::dev::mcp::{
    commands::read::availability::running_and_caught,
    facts::GameFacts,
    wire::{
        act::{ActRefusalNet, SelectReply},
        token::GangerToken,
    },
};

/// What a token that names no living ganger answers.
const UNKNOWN_TOKEN: SelectReply = SelectReply::Refused {
    reason: ActRefusalNet::UnknownToken,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActSelectArgs {
    /// Token of the ganger to select, as `battle.roster` reports it.
    ganger: GangerToken,
}

pub(crate) struct ActSelect;

impl McpCommand for ActSelect {
    type Args = ActSelectArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = SelectReply;

    const NAME: CommandName = CommandName::from_static("act.select");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select one ganger by token, the same path a left click on it takes. A token naming no \
         living ganger is refused UnknownToken. The game only ever selects a ganger the player \
         commands, so a token for someone else leaves the selection where it was; the reply says \
         who is selected once the frame has run.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_select.in_set(ActCommandSystems::Claim),
                settle_act_select.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_select(
    mut queue: ResMut<PendingQueue<CommandCall<ActSelect>>>,
    mut deferred: ResMut<DeferredReplies<ActSelect>>,
    mut pending: ResMut<PendingActIntent>,
    gangers: Query<Option<&LifeState>, With<Faction>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<ActSelect>(&mut queue) {
        let Some(actor) = a_ganger(&gangers, args.ganger) else {
            responder.answer(&UNKNOWN_TOKEN);
            continue;
        };
        pending.push(ActIntent::Select(actor));
        deferred.park(responder, ());
    }
}

fn settle_act_select(
    selected: Option<Res<SelectedShooter>>,
    mut deferred: ResMut<DeferredReplies<ActSelect>>,
) {
    settle_selects::<ActSelect>(selected.as_deref(), &mut deferred);
}
