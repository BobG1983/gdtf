//! Unit tests for the `procgen` packer core (GTW-424) — anchor selection + opposite-side
//! fit (deterministic under a seeded [`ProcgenRng`](crate::rng::ProcgenRng)), the 1-cell
//! seam margin (no abutting), and the connectivity assertion (holds for a valid placement,
//! fails closed on an impossible one). Wiring only: `mod` declarations, no test bodies.

mod anchor;
mod assembler;
mod packer;
mod seam;
