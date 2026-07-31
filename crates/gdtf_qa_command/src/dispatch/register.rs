//! Wiring one command — and a whole host's list — into an `App`.

use bevy::prelude::*;
use gdtf_net_qa_transport::{PendingQueue, sweep_pending};

use super::{
    CommandCall, CommandInbox, DeferredReplies, QaCommandSystems, claim_calls, sweep_deferred,
};
use crate::command::{ErasedCommand, QaCommand};

/// Wire one command: its typed queue, its deferral parking, its decode step, the two
/// deadline sweeps, and its own handler.
///
/// [`init_resource`](App::init_resource) is idempotent, so the shared
/// [`CommandInbox`] is initialised here rather than in a plugin a host could forget —
/// registering any command is enough to make the whole path exist.
///
/// It also chains [`Route`](QaCommandSystems::Route) before
/// [`Claim`](QaCommandSystems::Claim) in [`Update`]. That ordering is not the host's to
/// choose — a claim that ran before the router filled the [`CommandInbox`] would leave every
/// call a frame late — and declaring it here means a host cannot forget it. Repeating the
/// declaration once per command is harmless: it is the same ordering edge each time.
///
/// Both sweeps go in [`Last`], not beside the handlers in [`Update`]. A sweep that shared
/// a schedule with the handler it guards would be UNORDERED against it (bevy-traps #3), and
/// a sweep is meaningless before the frame's handlers have had their chance.
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

/// Wire a whole host's command list — ONE walk of the same slice the catalogue and
/// admission walk.
///
/// A command added to that list is advertised, admissible and wired by the same edit, and
/// there is no second list to fall out of step with.
pub fn register_command_set<F>(app: &mut App, commands: &[&dyn ErasedCommand<F>]) {
    for command in commands {
        command.register(app);
    }
}
