//! Typed pending call sitting in a Bevy queue.

use cobalt_mcp_transport::PendingQueue;

use super::CommandResponder;
use crate::command::QaCommand;

/// One admitted call with deserialized args for command `C`.
pub struct CommandCall<C: QaCommand> {
    args: C::Args,
}

impl<C: QaCommand> CommandCall<C> {
    /// Build a call from args.
    #[must_use]
    pub const fn new(args: C::Args) -> Self {
        Self { args }
    }

    /// Borrow the args.
    #[must_use]
    pub const fn args(&self) -> &C::Args {
        &self.args
    }

    /// Take the args.
    #[must_use]
    pub fn into_args(self) -> C::Args {
        self.args
    }
}

impl<C: QaCommand> core::fmt::Debug for CommandCall<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({:?})", C::NAME.as_str(), self.args)
    }
}

/// Drain ready calls and pair each with a typed responder.
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
