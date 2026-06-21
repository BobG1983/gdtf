//! Tests for [`has_los`](crate::los::has_los), split by acceptance criterion. Shared
//! fixtures live in [`support`]. No `App`, no RNG — hand-built grids (the sim-unit
//! idiom).

mod support;

mod asymmetry;
mod can_see;
mod clear;
mod corpse;
mod cover_band;
mod degenerate;
mod determinism;
mod facing_neutral;
mod parity;
