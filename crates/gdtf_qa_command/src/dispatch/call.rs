//! [`CommandCall`] — one decoded call waiting for its command's handler — and the typed
//! drain a handler reads it with.

use gdtf_net_qa_transport::PendingQueue;

use super::CommandResponder;
use crate::command::QaCommand;

/// One decoded call waiting for its command's handler.
///
/// It exists as its own type so the per-command
/// [`PendingQueue`] is `PendingQueue<CommandCall<C>>` — a distinct resource per command,
/// which is what lets each handler drain only its own work and lets the shared deadline
/// sweep be registered once per command with no knowledge of what the command is.
pub struct CommandCall<C: QaCommand> {
    /// The decoded arguments.
    args: C::Args,
}

impl<C: QaCommand> CommandCall<C> {
    /// Wrap a decoded argument record as a queued call.
    #[must_use]
    pub const fn new(args: C::Args) -> Self {
        Self { args }
    }

    /// The decoded arguments.
    #[must_use]
    pub const fn args(&self) -> &C::Args {
        &self.args
    }

    /// Take the decoded arguments, consuming the call.
    #[must_use]
    pub fn into_args(self) -> C::Args {
        self.args
    }
}

/// The [`Debug`] impl is hand-written: a derive would demand `C: Debug`, and `C` is a unit
/// struct nobody prints. The queue's deadline sweep only needs the ARGS to be printable,
/// which is exactly what [`QaCommand::Args`]'s `Debug` bound guarantees.
impl<C: QaCommand> core::fmt::Debug for CommandCall<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({:?})", C::NAME.as_str(), self.args)
    }
}

/// Drain this command's queue as typed calls with typed responders.
///
/// The handler never sees a raw `Responder` and never sees JSON — it gets `C::Args` and a
/// [`CommandResponder<C>`] that accepts only `&C::Reply`.
///
/// Call it behind a `queue.is_empty()` early-out: it takes `&mut` and so dirties the
/// resource's change-detection flag, and on a host with fifty commands almost every frame
/// has nothing to drain.
#[must_use]
pub fn take_calls<C: QaCommand>(
    queue: &mut PendingQueue<CommandCall<C>>,
) -> Vec<(C::Args, CommandResponder<C>)> {
    queue
        .drain_ready()
        .into_iter()
        .map(|(call, responder)| (call.into_args(), CommandResponder::new(responder)))
        .collect()
}
