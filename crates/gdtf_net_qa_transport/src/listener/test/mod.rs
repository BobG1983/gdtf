//! Unit tests for the loopback listener (GTW-736; lifted in GTW-803).
//!
//! - [`bind`] — the bind policy's ephemeral-port readback.
//! - [`socket`] — the session suite's real-listener / real-socket fixtures.
//! - [`session`] — the GTW-940 handshake gate: what is answered in the listener thread and
//!   what reaches the host inbox.

mod bind;
mod session;
mod socket;
