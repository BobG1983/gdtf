use gdtf_net_qa_transport::PendingQueue;

use super::CommandResponder;
use crate::command::QaCommand;

pub struct CommandCall<C: QaCommand> {
        args: C::Args,
}

impl<C: QaCommand> CommandCall<C> {
        #[must_use]
    pub const fn new(args: C::Args) -> Self {
        Self { args }
    }

        #[must_use]
    pub const fn args(&self) -> &C::Args {
        &self.args
    }

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
