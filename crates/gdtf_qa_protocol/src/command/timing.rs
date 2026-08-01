//! [`CommandTiming`] — when a command produces its reply (GTW-942).

use serde::{Deserialize, Serialize};

/// When a command produces its reply, relative to the frame its handler claims the call.
///
/// A catalogue row publishes this so a client knows, before it calls, whether the answer
/// comes back on the claiming frame or after the host has waited for something. It is a
/// DECLARATION by the command, not a measurement: a command that parks its responder in
/// `gdtf_qa_command`'s `DeferredReplies` resource says [`Deferred`](Self::Deferred) here,
/// and one that answers inside its handler says [`Immediate`](Self::Immediate).
///
/// Distinct from [`CommandAvailability`](crate::command::CommandAvailability): availability
/// is read from the frame's facts and changes as the app moves, while timing is a fixed
/// property of how the command is written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandTiming {
    /// The handler answers on the frame it claims the call.
    #[default]
    Immediate,
    /// The handler parks the reply and answers on a later frame.
    Deferred,
}

impl CommandTiming {
    /// Every timing, in declaration order.
    ///
    /// The list the round-trip suite walks to prove each value survives the wire; a new
    /// timing that is not listed here fails that test.
    pub const ALL: [Self; 2] = [Self::Immediate, Self::Deferred];
}
