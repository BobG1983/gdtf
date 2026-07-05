//! GTW-507 (child GTW-37c of GTW-37) — the LIVE melee ACT: a TU-costed close-combat strike
//! from 8-adjacency with clear LOS against an alive opposing ganger, resolved through the §7
//! opposed-Fight → §5 damage → §6 wound synthesis. Proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `MeleeRequested` (the same message the input seam writes).
//!
//! The acceptance criteria (C6 a / b / c / e):
//!
//! - **a — connect applies damage** (forced): an attacker 8-adjacent + in-LOS of an alive
//!   opposing ganger, on a FORCED connect (variance 0 + a zero-Fight defender → the degenerate
//!   `def ≤ 0` connect at `mult_max`), takes the target's HP DOWN, records a Wound, spends the
//!   attacker's TU, AND emits a `MeleeResolved`. PIN-DISCRIMINATING (fails if any link unwired).
//! - **b — miss applies nothing**: with the opposed roll LOST (variance 0 + a zero-Fight
//!   ATTACKER vs a Fight-positive defender → `atk ≤ def`, no connect), the target loses NO HP,
//!   records NO Wound, and NO `MeleeResolved` is emitted (a clean miss) — yet the attacker's TU
//!   IS spent (a swing costs TU whether or not it lands).
//! - **c — the gates** (each a discriminating case): a NON-ADJACENT target, a LOS-BLOCKED
//!   target, a SAME-FACTION (ally) target, and a DEAD target each produce NO melee (no HP loss,
//!   no `MeleeResolved`).
//! - **e — in-engine QA evidence (headless)**: the connect case proves the act resolves AND the
//!   `MeleeResolved` FX signal emits end-to-end on the real runtime path.
//!
//! DETERMINISM: a seeded battle RNG + a degenerate Fight on one side, so the outcome is
//! variance-INDEPENDENT (no `variance 0` — `opposed_fight` draws `random_range(1−v..1+v)`, which
//! is an empty range at `v == 0`). A ZERO-Fight DEFENDER drives the §7 degenerate `def ≤ 0`
//! connect (guaranteed connect at `mult_max`, any variance); a ZERO-Fight ATTACKER drives a
//! guaranteed MISS (`atk == 0 ≤ def > 0`, any variance) — the connect / miss outcome is a pure
//! function of the two gangers' Fights, with no brittle tunable-magnitude assert.
//!
//! HARNESS NOTE (the gtw468 / gtw355 idiom): the sim crate is the LOW crate, so it cannot dev-dep
//! `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

mod connect;
mod gates;
mod harness;
mod misses;
