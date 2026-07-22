//! GTW-541 (CORE of GTW-41) — `AoE` / `HitType` on the LIVE fire path: a weapon whose fired
//! mode carries a non-`Single` [`HitType`] applies its template at the shot's impact cell
//! and strikes EVERY occupant the template covers through the EXISTING
//! `resolve_and_apply` damage path — proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `FireRequested` (the same message the input layer writes).
//!
//! The clause contract this covers:
//!
//! - **`AoE` damages every occupant in radius**: a weapon authored with `HitType::Blast`
//!   fired at an impact cell damages EVERY ganger in the blast radius (HP / Wounds change
//!   on MULTIPLE targets from ONE shot) — the direct target AND the splash occupants,
//!   faction-blind (friendly fire). PIN-DISCRIMINATING (fails if the splash is unwired).
//! - **`HitType::Single` is unchanged**: a `Single` weapon fired at the same cluster
//!   damages ONLY the direct target — a bystander in an adjacent cell is untouched (no
//!   splash, the identity property).
//! - **Determinism**: the same `BattleSeed` reproduces an IDENTICAL multi-target outcome
//!   (the per-target HP after the blast is identical across two runs of the same seed).
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / bystander-untouched /
//! seed-reproducibility — never a specific damage number.
//!
//! HARNESS NOTE (the `melee_act` / `melee_cover_smash` idiom): the sim crate is the LOW
//! crate, so it cannot
//! dev-dep `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom
//! drives `setup_battle_on_request` via a `SetupBattleRequested` message against a
//! `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT
//! production wiring.

mod harness;
mod hit_types;
