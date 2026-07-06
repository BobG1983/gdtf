//! GTW-209 (E10.7): the headless end-to-end bootstrap CAPSTONE — a battle
//! constructs AND runs end-to-end under `MinimalPlugins`, with NO production code
//! of its own. It consumes ONLY what the prior E10 slices landed: the E10.1
//! `gdtf_app → gdtf_battle_sim` Cargo edge (so this crate can name sim types), the
//! E10.0 [`SimSystems::Simulate`] band, the E10.2 `*Requested` message contract +
//! per-act dispatch, the E10.5 `BattleSimPlugin` (Generation seeds the RNG streams
//! from the chosen [`BattleSeed`] source + inserts [`CombatTuning`] + runs
//! `setup_battle`), and the GTW-212 [`BattleInProgress`]-gated battle-wide dispatch
//! that E10.6 keeps live across `BattleRunning`.
//!
//! PROOF MIGRATION (GTW-324): this file is now built on the shared test
//! architecture — the drive-to-battle sequence is the crate-central
//! [`gdtf_test_utils::BattleAppBuilder`], and the authored battlefield + fixture
//! gangers come from the canonical [`gdtf_battle_sim::test_support`] builders
//! ([`GangerSpawnBuilder`] / [`SituationBuilder`]). It no longer hand-rolls a
//! `capstone_app`, its weapon/armor registries, or its `ganger_at` — those are the
//! shared builders the `BattleAppBuilder` seeds. The drive STOPS at
//! `BattleScapeState::BattleRunning` (never `AfterMath`).
//!
//! These are *pin-discriminating* tests: each `#[test]` re-encodes one acceptance
//! criterion as a before≠after / equality RELATION (never a pinned tunable magnitude),
//! so a regression in the E10 wiring turns the test red.
//!
//! NO function in this file takes `&mut World`/`&World`; every `app.world_mut()` /
//! `app.world()` call is in the TEST BODY (the established `gdtf_app` test idiom). No
//! `Camera`, no `Window`, no `gdtf_battle_presenter`, no `gdtf_battle_input`, no
//! `*Resolved` type, and the drive STOPS at `BattleScapeState::BattleRunning` (never
//! `AfterMath` / `AfterMathState`).

mod acts;
mod construct;
mod harness;
