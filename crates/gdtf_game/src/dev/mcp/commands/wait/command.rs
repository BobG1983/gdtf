use core::time::Duration;

use bevy::{ecs::message::Messages, prelude::*};
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredBudget, DeferredReplies, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::turn::TurnStarted;
use serde::{Deserialize, Serialize};

use super::probe::{TurnChangeCount, WaitProbe, count_turn_changes};
use crate::dev::mcp::{
    facts::GameFacts,
    wire::{AppPhaseNet, WaitConditionNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WaitArgs {
    condition: WaitConditionNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct WaitReply {
    condition: WaitConditionNet,
    phase:     AppPhaseNet,
}

/// What one parked `wait` is holding out for.
pub(crate) struct WaitTicket {
    condition:   WaitConditionNet,
    parked_with: TurnChangeCount,
}

pub(crate) struct Wait;

impl QaCommand for Wait {
    type Args = WaitArgs;
    type Facts = GameFacts;
    type Parked = WaitTicket;
    type Reply = WaitReply;

    const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(120));
    const NAME: CommandName = CommandName::from_static("wait");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Hold the reply until one named condition becomes true: CaughtUp, Phase, LogAtLeast, \
         WalkComplete, TurnChanged, BattleDecided or GenerationComplete. A condition that never \
         holds answers Timeout; a name this host does not offer is refused BadArguments with the \
         argument shape attached.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.init_resource::<TurnChangeCount>();
        app.add_systems(
            Update,
            (
                count_turn_changes.run_if(resource_exists::<Messages<TurnStarted>>),
                handle_wait,
            )
                .chain()
                .after(QaCommandSystems::Claim),
        );
    }
}

/// Shrink `wait`'s parking budget so a test can watch an unmet condition expire.
#[cfg(feature = "headless_test")]
pub fn shorten_wait_budget(app: &mut App, budget: DeferredBudget) {
    if let Some(mut parked) = app.world_mut().get_resource_mut::<DeferredReplies<Wait>>() {
        parked.set_budget(budget);
    }
}

fn handle_wait(
    probe: WaitProbe,
    mut queue: ResMut<PendingQueue<CommandCall<Wait>>>,
    mut deferred: ResMut<DeferredReplies<Wait>>,
) {
    if !queue.is_empty() {
        let parked_with = probe.turn_changes();
        for (args, responder) in take_calls::<Wait>(&mut queue) {
            deferred.park(
                responder,
                WaitTicket {
                    condition: args.condition,
                    parked_with,
                },
            );
        }
    }
    if deferred.is_empty() {
        return;
    }
    let phase = probe.phase();
    let delivered = deferred.answer_resolved(|ticket| {
        if *probe.met(ticket.condition, ticket.parked_with) {
            Some(WaitReply {
                condition: ticket.condition,
                phase,
            })
        } else {
            None
        }
    });
    if *delivered > 0 {
        debug!(delivered = *delivered, "mcp: wait conditions came true");
    }
}
