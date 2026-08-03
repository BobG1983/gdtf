//! Admit, claim, reply, and schedule command calls on a Bevy app.

/// Decide whether a named command may run.
pub mod admit;
/// Typed pending call payload.
pub mod call;
/// Move inbox entries into typed pending queues.
pub mod claim;
/// Parked replies that settle across frames.
pub mod deferred;
/// Shared inbox of admitted but unclaimed calls.
pub mod inbox;
/// Register command systems on a Bevy app.
pub mod register;
/// Protocol replies for unknown or unavailable commands.
pub mod reply;
/// Typed wrapper around a transport responder.
pub mod responder;
/// System sets for routing and claiming.
pub mod schedule;

pub use admit::{Admission, CommandRefusal, admit};
pub use call::{CommandCall, take_calls};
pub use claim::{bad_arguments, claim_calls};
pub use deferred::{
    DEFERRED_BUDGET, DeferredBudget, DeferredDelivery, DeferredReplies, DeliveredCount,
    sweep_deferred,
};
pub use inbox::{AdmittedCall, CommandInbox};
pub use register::{register_command, register_command_set};
pub use reply::{unavailable_reply, unknown_reply};
pub use responder::CommandResponder;
pub use schedule::QaCommandSystems;

#[cfg(test)]
mod test;
