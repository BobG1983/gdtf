//! Admit, claim, reply, and schedule command calls on a Bevy app.

/// Decide whether a named command may run.
pub mod admit;
/// Typed pending call payload.
pub mod call;
/// Host-side drain that takes the held replies' shots.
pub mod capture_drain;
/// Replies held until the capture rider's screenshot lands.
pub mod capture_hold;
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
/// Admit, hold, or refuse one incoming call.
pub mod route;
/// System sets for routing and claiming.
pub mod schedule;
/// Calls held while their command's admission is re-tested.
pub mod waiting;

pub use admit::{Admission, CommandRefusal, admit};
pub use call::{CommandCall, take_calls};
pub use capture_drain::drive_rider_captures;
pub use capture_hold::{CaptureHolds, CaptureTicket, RiderShot, ShotRequest, poll_capture_holds};
pub use claim::{bad_arguments, claim_calls};
pub use deferred::{
    DEFERRED_BUDGET, DeferredBudget, DeferredDelivery, DeferredReplies, DeliveredCount,
    sweep_deferred,
};
pub use inbox::{AdmittedCall, CommandInbox};
pub use register::{register_command, register_command_set, register_riders};
pub use reply::{unavailable_reply, unknown_reply};
pub use responder::CommandResponder;
pub use route::{CallQueues, IncomingCall, retest_waiting, route_call};
pub use schedule::QaCommandSystems;
pub use waiting::{WaitingAge, WaitingCall, WaitingCalls, WaitingSince};

#[cfg(test)]
mod test;
