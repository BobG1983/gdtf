//! GTW-546 (child GTW-41d of GTW-41) — ARCED / LOBBED fire + grenades: a per-weapon
//! `TrajectoryStyle::Arc` grenade is THROWN along a deterministic parabola that clears
//! same-level cover, passes holes / windows, and is BLOCKED by an intact roof; on landing it
//! fans a GTW-541 `HitType::Blast` at the landing cell through the EXISTING
//! `resolve_and_apply` damage path. Proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `ThrowGrenadeRequested` (the same message the input seam writes), plus pure
//! `march_arc` unit tests for the deterministic arc geometry.
//!
//! The clause contract this covers:
//!
//! - **`TrajectoryStyle` serde default `Straight`** — a weapon `.ron` that omits `trajectory:`
//!   parses as `Straight` (existing weapons unchanged); an authored `trajectory: Arc` parses
//!   as `Arc` (the identity property, the GTW-541 `HitType::Single` precedent).
//! - **Arc blocked by intact roof / passes holes** — `march_arc` from a higher thrower down
//!   onto a target under an intact `SlabState::Present` roof STOPS at the roof (a `Slab`
//!   landing at the roof cell, NOT the target); with a `SlabState::Destroyed` hole it PASSES
//!   and lands ON the target. Same-level lobs clear cover (never self-block on a same-level
//!   roof).
//! - **Blast hits room occupants through the roof hole** — a thrown Arc grenade lobbed at a
//!   room with a roof HOLE lands inside and its `HitType::Blast` DAMAGES the occupants (HP /
//!   Wounds drop). PIN-DISCRIMINATING (fails if the throw / blast is unwired).
//! - **Blast blocked by intact roof** — the SAME throw under an intact roof lands on the roof
//!   (a different cell / storey than the occupants), so the room occupants are UNTOUCHED.
//! - **Blind throw** — a thrower NOT FACING its target still resolves the throw (no LOS /
//!   facing / arc gate for an `Arc` weapon).
//! - **Determinism** — `march_arc` is a pure function (two identical calls agree); the arc
//!   geometry draws no RNG.
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / untouched / landing-cell —
//! never a specific damage number.
//!
//! HARNESS NOTE (the gtw541 idiom): the sim crate is the LOW crate, so it cannot dev-dep
//! `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

mod arc_march;
mod harness;
mod throw_resolution;
