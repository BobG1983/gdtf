//! GTW-526 (child of GTW-41) — the SUPPRESSION CORE: a ganger fired near by an OPPONENT
//! is suppressed (a worse reactor that auto-drops behind cover), symmetric across both
//! factions, cleared at its own turn-start. Proven on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band runtime (the gtw468
//! reaction-harness idiom — the sim crate cannot dep on `gdtf_test_utils` without a
//! dependency cycle, so it drives `SetupBattleRequested` against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app, the exact production wiring).
//!
//! The clauses under test:
//!
//! - **(a) producer** — an opposing `FireRequested` marks an IN-radius opposing ganger
//!   `Suppressed`, not an OUT-of-radius one, not a SAME-faction one.
//! - **(b) determinism** — a suppressed reactor does NOT interrupt AND consumes ZERO
//!   `ReactionRng` draws: after the tick the world's `ReactionRng` stream sits at its
//!   INITIAL position (identical first draw to a fresh stream from the same seed), while
//!   an unsuppressed control reactor DOES interrupt and advances the stream.
//! - **(c) clear cadence** — a suppressed unit stays suppressed through the opponent's
//!   turn and loses `Suppressed` at its OWN faction's `TurnStarted`.
//! - **(d) auto-stance** — a freshly-suppressed unit auto-drops (low cover → Prone, mid
//!   cover → Crouching, no cover → unchanged) with NO TU charged.
//! - **(e) idempotent refresh** — a second opposing shot on an already-suppressed unit
//!   emits `SuppressionApplied` ONCE (not twice) and does not stack.

mod application;
mod harness;
mod suppressed_effects;
