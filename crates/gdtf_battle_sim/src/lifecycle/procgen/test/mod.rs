//! Unit tests for the `procgen` packer core — anchor selection + opposite-side fit
//! (deterministic under a seeded [`ProcgenRng`](crate::rng::ProcgenRng)), the 1-cell seam
//! margin (no abutting), and the connectivity assertion (GTW-424); plus the GTW-427 random
//! same-theme fill pass ([`fill`]: C1/C2/C3 termination + no-fit pad + determinism) and the
//! OQ-6 [`ProcgenTuning`](crate::procgen::ProcgenTuning) knobs ([`tuning`]); plus the
//! GTW-431 emit step ([`emit`]: the deterministic seed harness emits a terrain-equal level
//! for the same seed and a terrain-different level for different seeds (C2), and the emitted
//! [`Situation`](crate::situation::Situation) is in-bounds + connected (C3)). Wiring only:
//! `mod` declarations, no test bodies.

mod anchor;
mod assembler;
mod emit;
mod fill;
mod packer;
mod seam;
mod tuning;
