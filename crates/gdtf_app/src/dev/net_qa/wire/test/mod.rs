//! Tests for the GAME host's own wire types (GTW-942, GTW-943, GTW-944).
//!
//! - [`phase`] — every arm of the five state mirrors, and the nesting
//!   [`AppPhaseNet`](super::AppPhaseNet) carries.
//! - [`act`] — exhaustive per-variant round-trip + parity pins for the act vocabulary.
//! - [`cell`] — round-trip and wire-text pins for the grid coordinates.
//! - [`log`] — round-trip and wire-text pins for the act-log vocabulary.
//! - [`drive`] — round-trip and wire-text pins for the focus-step and procgen-stepper
//!   commands.
//! - [`scalars`] — round-trip AND compact-text pins for the entity tokens, the pointer
//!   vocabulary and the scalar handles.
//! - [`schema`] — every type's derived `JsonSchema` is usable JSON, and every named-field
//!   struct refuses unknown properties.
//! - [`coverage`] — the pin that keeps the two suites above complete: a type added to
//!   `wire/` without a round-trip case and a schema case fails here.
//! - [`support`] — the shared compact-RON round-trip assertion.

mod act;
mod cell;
mod coverage;
mod drive;
mod log;
mod phase;
mod scalars;
mod schema;
mod support;

pub(in crate::dev::net_qa::wire) use support::assert_ron_round_trip;
