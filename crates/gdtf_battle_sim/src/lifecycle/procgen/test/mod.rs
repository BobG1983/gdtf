//! Unit tests for the `procgen` packer core — anchor selection + opposite-side fit
//! (deterministic under a seeded [`ProcgenRng`](crate::rng::ProcgenRng)), the 1-cell seam
//! margin (no abutting), and the connectivity assertion (GTW-424); plus the GTW-427 random
//! same-theme fill pass ([`fill`]: C1/C2/C3 termination + no-fit pad + determinism) and the
//! OQ-6 [`ProcgenTuning`](crate::procgen::ProcgenTuning) knobs ([`tuning`]); plus the
//! GTW-431 emit step ([`emit`]: the deterministic seed harness emits a terrain-equal level
//! for the same seed and a terrain-different level for different seeds (C2), and the emitted
//! [`Situation`](crate::situation::Situation) is in-bounds + connected (C3)); plus the
//! GTW-582 C3(d) engagement-time finding pins ([`findings`]: the nil-sentinel theme pour and
//! the fail-open unresolved-piece pour each ride back on
//! [`EmittedLevel::findings`](crate::procgen::EmittedLevel), deduplicated — never silent);
//! plus the GTW-655 staged-driver pins ([`staged`]: a stepped-to-completion drive produces an
//! IDENTICAL result to a one-shot `generate_level` call, each `advance` runs exactly one
//! stage, and a finished/failed drive is idempotent under a repeat `advance`); plus the
//! GTW-744 roster-deployment pins ([`deploy`]: [`deploy_rosters`](crate::procgen::deploy_rosters)
//! deploys deterministically by seed over the REAL `generate_level` zones, every placement is
//! valid/standable/distinct/in-zone across seeds, and a too-small zone fails closed with a
//! typed [`PackingError`](crate::procgen::PackingError)). Wiring only: `mod` declarations, no
//! test bodies.

mod anchor;
mod assembler;
mod deploy;
mod emit;
mod fill;
mod findings;
mod packer;
mod seam;
mod staged;
mod tuning;
