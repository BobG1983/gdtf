//! Register command systems and resources on a Bevy app.

use bevy::prelude::*;
use cobalt_mcp_transport::{PendingQueue, sweep_pending};
use cobalt_screenshot::CapturePipelinePlugin;

use super::{
    CaptureHolds, CaptureTicket, CommandCall, CommandInbox, DeferredReplies, McpCommandSystems,
    WaitingCalls, claim_calls, drive_rider_captures, poll_capture_holds, sweep_deferred,
};
use crate::command::{ErasedCommand, McpCommand};

/// Register the resources, the capture pipeline, and the systems the run riders need.
///
/// The host that gets the hold gets the queue that feeds it and the drain that empties it, so a
/// capture rider answers on every host rather than waiting out the caller's reply timeout.
pub fn register_riders(app: &mut App) {
    app.init_resource::<WaitingCalls>();
    app.init_resource::<CaptureHolds>();
    app.add_systems(Last, poll_capture_holds);
    if !app.is_plugin_added::<CapturePipelinePlugin<CaptureTicket>>() {
        app.add_plugins(CapturePipelinePlugin::<CaptureTicket>::new());
    }
    app.add_systems(Update, drive_rider_captures.after(McpCommandSystems::Claim));
}

/// Register inbox, queue, claim, sweep, and the command's own handler for `C`.
pub fn register_command<C: McpCommand>(app: &mut App) {
    app.configure_sets(
        Update,
        (McpCommandSystems::Route, McpCommandSystems::Claim).chain(),
    );
    app.init_resource::<CommandInbox>();
    app.init_resource::<PendingQueue<CommandCall<C>>>();
    app.insert_resource(DeferredReplies::<C>::with_budget(C::DEFERRED_BUDGET));
    app.add_systems(Update, claim_calls::<C>.in_set(McpCommandSystems::Claim));
    app.add_systems(Last, (sweep_pending::<CommandCall<C>>, sweep_deferred::<C>));
    C::register_handler(app);
}

/// Register every command in a set, plus the rider wiring the router needs.
pub fn register_command_set<F>(app: &mut App, commands: &[&dyn ErasedCommand<F>]) {
    register_riders(app);
    for command in commands {
        command.register(app);
    }
}
