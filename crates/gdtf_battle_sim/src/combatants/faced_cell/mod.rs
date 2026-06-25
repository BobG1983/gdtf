//! The faced-cell geometry helper — the cell a shooter's
//! [`Facing`](crate::ganger::Facing) looks at.
//!
//! The §1a brace gate (`docs/combat/resolution.md` line 26: "+30 when **the faced
//! cell's** cover height suits the stance") needs the cell whose cover height the
//! stability composer (E4.3) reads. That is the ground cell one step along the
//! shooter's facing, on the shooter's **own** storey. [`faced_cell`] is that pure
//! geometry: shooter [`Position`](crate::ganger::Position) +
//! [`Facing`](crate::ganger::Facing) → the faced ([`Cell`](crate::metric::Cell),
//! [`Level`](crate::metric::Level)). No `World` access, render-free, **zero pixels** —
//! it composes only the [`crate::metric`] sim-unit conversions
//! (`docs/combat/battle-space.md` §"Sub-cell precision on the ground plane").
//!
//! ## The diagonal subtlety
//!
//! [`Direction::forward_step`](crate::ganger::Direction::forward_step) returns a
//! **unit** `Vec3`, so a diagonal step is `(±1/√2, ±1/√2) ≈ (±0.707, ±0.707)`, not
//! `(±1, ±1)`. Flooring that step added to the bare integer cell **corner** would fail
//! to advance a diagonal (`0.0 + 0.707 = 0.707 → floor 0`). The grounded derivation
//! steps from the cell **center**: [`cell_center`](crate::metric::cell_center) adds the
//! `+0.5` x/y offset, so the diagonal crosses the boundary (`0.5 + 0.707 = 1.207 →
//! floor 1`) and a cardinal advances on its one axis (`0.5 + 1.0 = 1.5 → floor 1`; the
//! un-stepped axis `0.5 + 0.0 = 0.5 → floor 0`).
//! [`pos_to_cell`](crate::metric::pos_to_cell) floors — never rounds — so it advances
//! correctly even at a negative cell. Reusing this one metric-conversion path (rather
//! than re-deriving per-axis signs) keeps the helper aligned with the rest of the shot
//! pipeline.

mod helper;
#[cfg(test)]
mod test;

pub use helper::faced_cell;
