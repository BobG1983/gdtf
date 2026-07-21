//! The QA outbox — the GTW-694 architecture's T6, projecting the GTW-727 act log onto the
//! curated wire event stream (GTW-739).
//!
//! A `GetOutput` is answered by PROJECTING the sim-owned, non-destructive
//! [`ActLog`](gdtf_battle_sim::act_log::ActLog) — the ONE ordered source GTW-727 built —
//! through a single curation into the bevy-free wire
//! [`NetEvent`](gdtf_qa_protocol::events::NetEvent) vocabulary. It is NOT a second event
//! feed: there is no message tap here, only a read of the log the presenter's playback
//! cursor also reads, over an independent cursor that never consumes the ring.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`map`] — the pure, exhaustive, wildcard-free
//!   [`net_event_for`](map::net_event_for) projection (one act-log entry → its curated wire
//!   event, or `None` — the `Option` IS the curation).
//! - [`cursor`] — the outbox's own [`QaOutputCursor`](cursor::QaOutputCursor) read position.
//! - [`pump`] — the [`drive_output`](pump::drive_output) service that drains the routed
//!   `GetOutput` queue and replies a real event batch.

mod cursor;
mod map;
mod pump;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use cursor::QaOutputCursor;
pub(in crate::dev::net_qa) use pump::{drive_output, reset_output_cursor_when_idle};
