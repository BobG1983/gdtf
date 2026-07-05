//! The **fill pass** — the GTW-427 random same-theme fill that pours connective `Fill`
//! prefabs into the free space the GTW-424 player + enemy placement leaves, then pads the
//! remaining dead space with open `default_floor` lanes.
//!
//! This is the SECOND stage of the staged assembler (424 placement -> 427 fill -> 431
//! emit/trigger). It builds on the [`Placement`](super::assembler::Placement) GTW-424
//! produced: it re-derives the packer free space by carving the two placed regions, then
//! draws random same-theme [`Fill`](crate::level::SpawnRole::Fill) prefabs from the
//! registry via [`ProcgenRng`](crate::rng::ProcgenRng) and packs them in — until the
//! coverage [`MinDensityFloor`](super::tuning::MinDensityFloor) is reached OR no more
//! prefab fits (C1/C2 termination).
//!
//! The pass runs three sub-passes against the three OQ-6 knobs ([`ProcgenTuning`](super::tuning::ProcgenTuning)):
//!
//! 1. **`FillLarge`** — place large fill prefabs (footprint area `>=`
//!    [`LargePrefabAreaThreshold`](super::tuning::LargePrefabAreaThreshold)) first, while
//!    the biggest contiguous free rectangles still exist.
//! 2. **Smaller-prefab fill** — place the remaining (sub-threshold) fill prefabs.
//! 3. **Dead-rect scatter** — scatter up to
//!    [`DeadRectScatterCount`](super::tuning::DeadRectScatterCount) micro-pieces into each
//!    `>= 4x4` leftover dead rect, breaking up big empty halls.
//!
//! # The no-fit fallback (RULED — GTW-424 connectivity ruling, user 2026-06-26)
//!
//! When no more prefab fits, the remaining dead space is **padded with `default_floor`
//! (open lanes)** — the playable area is NEVER shrunk. The dead-space-as-floor regions are
//! returned in [`FilledPlacement::dead_space`] so the GTW-431 emit step pours them as
//! `default_floor`; the fill never carves doorways, never walls anything off (OQ-4
//! connectivity-by-construction is preserved by the 1-cell seam every placement still
//! reserves — that seam lattice alone joins every open cell). This is the "chosen no-fit
//! fallback" the AC references.
//!
//! # Determinism
//!
//! The fill draws every prefab choice from the injected [`ProcgenRng`](crate::rng::ProcgenRng)
//! in a FIXED order (the sub-pass order above, and within each pass a deterministic
//! free-rect scan), so the same seed produces an identical fill (the determinism contract).
//! The loop is BOUNDED: every iteration either places a prefab (consuming free space) or
//! ends the pass, so it always terminates (C2 — no infinite loop).

mod outcome;
mod passes;
mod pipeline;

pub use outcome::FilledPlacement;
pub use pipeline::{fill_placement, fill_placement_with};
