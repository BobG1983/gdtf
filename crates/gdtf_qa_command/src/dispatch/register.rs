use bevy::prelude::*;
use gdtf_net_qa_transport::{PendingQueue, sweep_pending};

use super::{
    CommandCall, CommandInbox, DeferredReplies, QaCommandSystems, claim_calls, sweep_deferred,
};
use crate::command::{ErasedCommand, QaCommand};

pub fn register_command<C: QaCommand>(app: &mut App) {
    app.configure_sets(
        Update,
        (QaCommandSystems::Route, QaCommandSystems::Claim).chain(),
    );
    app.init_resource::<CommandInbox>();
    app.init_resource::<PendingQueue<CommandCall<C>>>();
    app.init_resource::<DeferredReplies<C>>();
    app.add_systems(Update, claim_calls::<C>.in_set(QaCommandSystems::Claim));
    app.add_systems(Last, (sweep_pending::<CommandCall<C>>, sweep_deferred::<C>));
    C::register_handler(app);
}

pub fn register_command_set<F>(app: &mut App, commands: &[&dyn ErasedCommand<F>]) {
    for command in commands {
        command.register(app);
    }
}
