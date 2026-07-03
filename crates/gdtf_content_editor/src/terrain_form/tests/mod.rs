//! In-crate tests for the TERRAIN form (GTW-474). Wiring only: `mod` declarations, no
//! test bodies.
//!
//! - [`form`] — the pure draft → def projection, the C3 RON round-trip on the loader's
//!   parser, the C2 footfall gate, and the GTW-566 picker derivation.
//! - [`kinds`] — the GTW-574 kind-identity pins: the canonical-discriminant round-trip
//!   (`From<TerrainPieceKind>` ∘ the choice → sim-kind projection is kind-level
//!   identity), the `SEGMENT_ORDER` completeness pin, the Emplacement projection, and
//!   the fail-closed missing-mounted-weapon rule.
//! - [`support`] — shared fixtures (the deterministic test UUID).
//!
//! The C4 in-engine / loader round-trip lives in the crate's integration test
//! (`tests/terrain_mode.rs`). Panic / expect-free per the workspace lints (`assert!` +
//! `let … else`; `unreachable!` only behind asserts).

mod form;
mod kinds;
mod support;
