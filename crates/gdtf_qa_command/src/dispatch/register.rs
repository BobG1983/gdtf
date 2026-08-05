//! Register command systems and resources on a Bevy app.

use bevy::prelude::*;
use gdtf_net_qa_transport::{PendingQueue, sweep_pending};

use super::{
    CommandCall, CommandInbox, DeferredReplies, QaCommandSystems, claim_calls, sweep_deferred,
};
use crate::command::{ErasedCommand, QaCommand};

/// Register inbox, queue, claim, sweep, and the command's own handler for `C`.
pub fn register_command<C: QaCommand>(app: &mut App) {
    app.configure_sets(
        Update,
        (QaCommandSystems::Route, QaCommandSystems::Claim).chain(),
    );
    app.init_resource::<CommandInbox>();
    app.init_resource::<PendingQueue<CommandCall<C>>>();
    app.insert_resource(DeferredReplies::<C>::with_budget(C::DEFERRED_BUDGET));
    app.add_systems(Update, claim_calls::<C>.in_set(QaCommandSystems::Claim));
    app.add_systems(Last, (sweep_pending::<CommandCall<C>>, sweep_deferred::<C>));
    C::register_handler(app);
}

/// Register every command in a set.
pub fn register_command_set<F>(app: &mut App, commands: &[&dyn ErasedCommand<F>]) {
    for command in commands {
        command.register(app);
    }
}
