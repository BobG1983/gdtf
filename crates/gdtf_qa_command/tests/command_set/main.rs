//! The command-set suite (GTW-941) — the whole abstraction proved on a BARE `App` with
//! fake commands, before any real command exists.
//!
//! Bare is the point. Nothing here needs the game, the editor, a socket or a plugin group:
//! if this passes, the plumbing works, and a host's job is reduced to a facts type, a list,
//! and a fifty-line router.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`support`] — the shared read helpers every case below uses.
//! - [`round_trip`] — admit → claim → decode → handle → typed reply.
//! - [`one_predicate`] — `admit` refuses exactly what `catalogue` marks unavailable.
//! - [`bad_arguments`] — a payload that will not decode is answered at claim time.
//! - [`unknown_name`] — an unknown name is answered with every known name.
//! - [`deferred`] — a parked reply settles later, and expires before the socket.
//! - [`stalled`] — a call no handler drains is answered by the pending queue's deadline.
//! - [`growth`] — adding a command moves no version.
//! - [`riders`] — a non-default `RunOptions` is refused `NotBuilt`.
//! - [`conformance`] — the two shared assertions, called from ANOTHER crate's test.

mod bad_arguments;
mod conformance;
mod deferred;
mod growth;
mod one_predicate;
mod riders;
mod round_trip;
mod stalled;
mod support;
mod unknown_name;
