//! Unit tests for the act log's own mechanics — the ring, the cursor, and the
//! transition-detection maps. Wiring only: `mod` declarations, no test bodies here.
//!
//! The BEHAVIOURAL tests (the recorder pass driven through a real app) live in the
//! `tests/act_log/` integration suite; these cover [`ActLog`](super::ActLog) in isolation,
//! where no app is needed at all.

mod ring;
mod transitions;
