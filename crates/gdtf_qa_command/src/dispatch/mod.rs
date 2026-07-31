//! Admission, decode, typed hand-off, deferral and registration — the path one `Run`
//! travels from the socket to a command's own handler.
//!
//! The order, once per call: a host's router [`admit()`]s it against that host's slice, parks
//! it in the [`CommandInbox`], and on the same frame [`claim_calls`] decodes it into the
//! command's typed [`PendingQueue`](gdtf_net_qa_transport::PendingQueue). The command's own
//! handler drains that queue with [`take_calls`] and answers through a
//! [`CommandResponder`]. A handler that cannot answer yet parks its responder in
//! [`DeferredReplies`] and answers on a later frame.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `admit` — [`Admission`], [`CommandRefusal`], and [`admit()`], the linear scan.
//! - [`inbox`] — [`CommandInbox`] and [`AdmittedCall`].
//! - [`call`] — [`CommandCall`] and [`take_calls`].
//! - [`claim`] — [`claim_calls`] and [`bad_arguments`].
//! - [`responder`] — [`CommandResponder`].
//! - [`reply`] — the two route-time reply shapers.
//! - [`deferred`] — [`DeferredReplies`] and [`sweep_deferred`].
//! - [`register`] — [`register_command`] and [`register_command_set`].
//! - [`schedule`] — [`QaCommandSystems`].

pub mod admit;
pub mod call;
pub mod claim;
pub mod deferred;
pub mod inbox;
pub mod register;
pub mod reply;
pub mod responder;
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
