//! The combat-event outbox vocabulary — [`NetEvent`] + [`EventBatch`] (GTW-734).
//!
//! A curated subset of the sim's combat-message vocabulary that a QA client polls via
//! [`GetOutput`](crate::envelope::QaRequest::GetOutput): the shots, moves, downs,
//! deaths, injuries, and turn transitions worth asserting on. The event enum
//! ([`net_event`]) mirrors the QA-relevant sim messages; the batch wrapper
//! ([`batch`]) carries the drained events plus a [`DroppedCount`] back-pressure
//! signal.

pub mod batch;
pub mod net_event;

pub use batch::{DroppedCount, EventBatch};
pub use net_event::{MoveRejectionNet, NetEvent, ShotKindNet};

#[cfg(test)]
mod test;
