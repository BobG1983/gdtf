//! GTW-355 (E7 · GTW-12g) — the COMMITTED step-by-step walk: an accepted move no longer
//! jumps to the destination; it walks the planned route ONE cell per tick, charging each
//! step atomically, bump-stopping on a live obstacle, and halting the moment an enemy is
//! revealed or a reaction shot interrupts. Proven end-to-end on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path.
//!
//! The four C7 behaviors:
//!
//! - **(a) BUMP-STOP** — a walk whose next cell becomes OCCUPIED after the route was
//!   planned stops at the last free step, charged ONLY the steps actually taken (no
//!   teleport, no co-location).
//! - **(b) §48 BIT-IDENTITY** — an UNINTERRUPTED walk's total charged TU equals the
//!   `find_path` `Path::total()` for that route, bit-for-bit (a wrong per-step cost source
//!   would break this — pin-discriminating).
//! - **(c) STOP-ON-REVEAL** — a walk that brings a previously-UNSEEN enemy into the squad
//!   VISIBLE set after a step halts immediately, charged only the steps taken.
//! - **(d) STOP-ON-INTERRUPT** — a SYNTHETIC `ReactionShotFired` mid-walk halts the walk,
//!   charged only the steps taken. (The reaction-fire PRODUCER is GTW-38-future; this slice
//!   builds the receiving hook only.)
//!
//! RELATIONS-ONLY, pin-discriminating, NO magnitude pins: TU is compared by relations
//! (`spent == 0` / `spent > 0` / `walk-TU == find_path total`), never against a shipped
//! move-cost magnitude.
//!
//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`", as in GTW-354):
//! the sim crate is the LOW crate — `gdtf_test_utils` depends on `gdtf_app` which depends
//! on `gdtf_battle_sim`, so a sim-crate dev-dep on `gdtf_test_utils` would be a dependency
//! CYCLE. The established sim-crate battle-integration idiom (`squad_fog_recompute` / `dispatch_move_constraints`) drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

mod charging;
mod harness;
mod interrupts;
