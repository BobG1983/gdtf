//! GTW-727 — the sim's ACT LOG, proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`/`Record` path.
//!
//! The log exists so a view can show acts one at a time without ever gating the sim. These
//! tests pin the four properties that makes possible:
//!
//! - **`reaction_provenance`** — the acceptance criterion itself (T1): two reactors
//!   interrupting one step produce two separately-identifiable reaction entries, in order,
//!   each naming who it interrupted. Plus T2: one reactor firing twice in a tick pairs each
//!   declaration with its OWN rounds, which is what makes T1 hold in the multi-interrupt
//!   case rather than by luck.
//! - **`determinism`** — the single-writer claim (T3): one seed, two runs, identical logs.
//!   Plus T5: wiring the recorder perturbs no seeded outcome, so it is provably pure
//!   exposure.
//! - **`spawn_quiet`** — the spawn-flood cure (T4): setting up a roster records nothing.
//!   Plus T7: a battle with no presenter runs to an outcome, so clause (d) is asserted
//!   rather than assumed.
//! - **`deed_coverage`** — the completeness bar (T8): a new deed fails to compile until it
//!   is consciously classified.
//!
//! The ring's own mechanics — sequence assignment, the non-destructive cursor read, and
//! capacity overflow dropping the oldest and counting it (T6) — need no app at all and live
//! as unit tests beside the type, in `src/act_log/test/ring.rs`.
//!
//! HARNESS NOTE: the sim crate is the LOW crate, so it cannot depend on `gdtf_test_utils`
//! (that would be a dependency cycle). These drive `setup_battle_on_request` via a
//! `SetupBattleRequested` message against a `MinimalPlugins` + `AssetPlugin` + `ScenePlugin`
//! + `BattleSimPlugin` app — the established sim-crate battle-integration idiom, and the
//!   EXACT production wiring.
//!
//! Wiring only.

mod deed_coverage;
mod determinism;
mod harness;
mod reaction_provenance;
mod spawn_quiet;
