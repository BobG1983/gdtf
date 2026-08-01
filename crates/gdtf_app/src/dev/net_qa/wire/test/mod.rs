//! Tests for the GAME host's own wire types (GTW-942, GTW-943).
//!
//! - [`phase`] — every arm of the five state mirrors, and the nesting
//!   [`AppPhaseNet`](super::AppPhaseNet) carries.
//! - [`act`] — exhaustive per-variant round-trip + parity pins for the act vocabulary.
//! - [`scalars`] — round-trip AND compact-text pins for the entity tokens, the pointer
//!   axes and the scalar handles.
//! - [`support`] — the shared compact-RON round-trip assertion.

mod act;
mod phase;
mod scalars;
mod support;

pub(in crate::dev::net_qa::wire) use support::assert_ron_round_trip;
