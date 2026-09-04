//! `wait` — hold the reply until one named editor condition comes true.

use core::time::Duration;

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, DeferredBudget, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use serde::{Deserialize, Serialize};

use super::probe::{EditorWaitProbe, RegistryChangeCounts, count_registry_changes};
use crate::mcp::{
    facts::EditorFacts,
    schedule::EditorMcpSystems,
    wire::{EditorPhaseNet, EditorWaitConditionNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorWaitArgs {
    condition: EditorWaitConditionNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorWaitReply {
    condition: EditorWaitConditionNet,
    phase:     EditorPhaseNet,
}

/// What one parked `wait` is holding out for, and the tallies it parked against.
pub(in crate::mcp) struct EditorWaitTicket {
    condition:   EditorWaitConditionNet,
    parked_with: RegistryChangeCounts,
}

pub(in crate::mcp) struct EditorWait;

impl McpCommand for EditorWait {
    type Args = EditorWaitArgs;
    type Facts = EditorFacts;
    type Parked = EditorWaitTicket;
    type Reply = EditorWaitReply;

    const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(120));
    const NAME: CommandName = CommandName::from_static("wait");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Hold the reply until one named condition becomes true: ChecksComplete, or \
         RegistryRearmed for one content family. A condition that never holds answers Timeout, \
         and a family this host does not offer is refused BadArguments with the argument shape \
         attached.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(_facts: &EditorFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.init_resource::<RegistryChangeCounts>();
        app.add_systems(
            Update,
            (count_registry_changes, handle_editor_wait)
                .chain()
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

/// Shrink `wait`'s parking budget so a test can watch an unmet condition expire.
pub fn shorten_editor_wait_budget(app: &mut App, budget: DeferredBudget) {
    if let Some(mut parked) = app
        .world_mut()
        .get_resource_mut::<DeferredReplies<EditorWait>>()
    {
        parked.set_budget(budget);
    }
}

fn handle_editor_wait(
    probe: EditorWaitProbe,
    mut queue: ResMut<PendingQueue<CommandCall<EditorWait>>>,
    mut deferred: ResMut<DeferredReplies<EditorWait>>,
) {
    if !queue.is_empty() {
        let parked_with = probe.registry_changes();
        for (args, responder) in take_calls::<EditorWait>(&mut queue) {
            deferred.park(
                responder,
                EditorWaitTicket {
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
            Some(EditorWaitReply {
                condition: ticket.condition,
                phase,
            })
        } else {
            None
        }
    });
    if *delivered > 0 {
        debug!(
            delivered = *delivered,
            "editor mcp: wait conditions came true"
        );
    }
}
