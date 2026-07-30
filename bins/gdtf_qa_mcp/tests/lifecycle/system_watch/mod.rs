//! What the REAL [`SystemOrphanWatch`](gdtf_qa_mcp::SystemOrphanWatch) observes and what it
//! actually signals (GTW-926).
//!
//! The manager's orphan DECISIONS are covered in [`orphan`](super::orphan) against a watch
//! whose stop is recorded rather than sent. This directory covers the other half — the
//! watch itself — because that is where the reported defect's recovery lives: the
//! hand-typed `kill -TERM 43744` this ticket replaces. Every test here signals a
//! placeholder process the test spawned itself, never the test runner. One concern per
//! file:
//!
//! - [`lookup`] — the lookup names the process holding a port.
//! - [`signal_target`] — the stop reaches the named process whether or not it leads a
//!   process group of its own.
//! - [`group_stop`] — the stop also reaches a process the named one spawned, which only
//!   the group target can carry.
//! - [`escalation`] — a process that ignores the graceful signal is killed, with the port,
//!   not the `kill` returning, deciding when the stop is done.
//! - [`placeholder`] — the spawning, process-group reading and exit-waiting helpers the
//!   test files above share.

mod escalation;
mod group_stop;
mod lookup;
mod placeholder;
mod signal_target;
