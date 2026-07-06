//! GTW-468 (the final child of GTW-38) — the LIVE reaction-fire trigger: when a ganger
//! ACTS in an opposing reactor's LOS, the §8 opposed check fires an interrupt from the
//! reactor's unspent TU, halts a walking actor, and consumes the reactor's per-turn cap.
//! Proven END-TO-END on the REAL `setup_battle_on_request` → `BattleSimPlugin`
//! `Simulate`-band path, driven THROUGH the move / brain / dispatch path (NOT a synthetic
//! `FireRequested`/`ReactionShotFired` emit).
//!
//! The acceptance criteria:
//!
//! - **AC1** — an actor (faction A) MOVING within LOS of a watcher (faction B, unspent TU,
//!   fresh cap) → the interrupt FIRES: the watcher's TU is debited AND a `ShotFired` from
//!   the watcher is observed on the actor's cell. PIN-DISCRIMINATING (fails if unwired).
//! - **AC2** — a walking actor HALTS on the (forced-success) interrupt: its `WalkInProgress`
//!   is removed and it stops short of its destination, via the LIVE `ReactionShotFired`.
//! - **AC3** — LOS gate: an actor acting OUTSIDE all opposing watchers' LOS produces NO
//!   interrupt; the SAME act inside LOS + arc + range DOES.
//! - **AC4** — the per-turn CAP bites: with cap == 1 a watcher interrupts ONCE this turn and
//!   not again, and reacts AGAIN next turn (the reset exercised through a turn cycle).
//! - **AC5** — faction symmetry: a player watcher interrupts an acting ENEMY during the
//!   enemy turn (the brain drives the enemy move) AND an enemy watcher interrupts an acting
//!   PLAYER during the player turn.
//! - **AC7** — the reaction shot is a NORMAL shot (no reaction damage modifier): it resolves
//!   through the normal `dispatch_fire` → `fire()` pipeline (a real `ShotFired` with a
//!   `HitReport`), never a bespoke reaction path.
//!
//! DETERMINISM: a seeded battle RNG, plus a tuning clamp `p_min == p_max == 1.0` to FORCE a
//! guaranteed interrupt where the test needs one (`rolls_interrupt` draws `roll ∈ [0,1)` and
//! compares `roll < 1.0`, always true — `clamp_probability` returns `1.0` when min == max).
//! AC3's no-interrupt case is forced by GEOMETRY (LOS blocked / out of range), not by the
//! probability. Structural facts only (TU debited, `WalkInProgress` removed, `ShotFired`
//! emitted), never brittle magnitudes.
//!
//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`", as in GTW-355): the
//! sim crate is the LOW crate — `gdtf_test_utils` depends on `gdtf_app` which depends on
//! `gdtf_battle_sim`, so a sim-crate dev-dep on `gdtf_test_utils` would be a dependency
//! CYCLE. The established sim-crate battle-integration idiom (`squad_fog_recompute` / `committed_walk`) drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

mod cap;
mod harness;
mod triggers;
