//! The typed pending queue + the frame-deadline sweep (GTW-736; lifted in GTW-803).
//!
//! A request the host cannot answer synchronously lands in a per-kind [`PendingQueue`]
//! where a later consumer claims it. Every entry carries a frame-deadline countdown; the
//! [`sweep_pending`] pump answers the client with a
//! [`Timeout`](gdtf_qa_protocol::message::QaError::Timeout) when it expires unclaimed — so
//! a request never leaves the client hanging. The payload types a queue holds are the
//! host's: this module is generic over `P` and never names one.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`deadline`] — the frames-of-grace budget and the per-entry countdown.
//! - [`queue`] — the [`PendingQueue`] FIFO and its entries.
//! - [`sweep`] — the [`sweep_pending`] system a host registers per payload type.

mod deadline;
mod queue;
mod sweep;

pub use queue::PendingQueue;
pub use sweep::sweep_pending;

#[cfg(test)]
mod test;
