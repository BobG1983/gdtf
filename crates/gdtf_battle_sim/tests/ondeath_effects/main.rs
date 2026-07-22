//! GTW-547 (on-death effects, child GTW-41g) — on the LIVE combat path: a ganger whose
//! wielded weapon authors an `OnDeathEffect::Explode` DIES to a shot and fans a GTW-541 `AoE`
//! blast at its body (adjacent gangers take damage); a piece of cover whose `TerrainDef`
//! authors an `OnDeathEffect::LeaveField` is DESTROYED by fire and leaves a GTW-545 field at
//! its cell. Both proven END-TO-END on the REAL `setup_battle_on_request` → `BattleSimPlugin`
//! `Simulate`-band path, driven THROUGH a buffered `FireRequested` (the same message the
//! input layer writes) — so the `resolve_on_death` resolver runs in its real schedule slot.
//!
//! The clause contract this covers:
//!
//! - **Explode fans an `AoE` blast; cells in radius take damage**: a ganger with an Explode
//!   on-death effect that dies to a shot damages an ADJACENT ganger (the headline test).
//!   PIN-DISCRIMINATING (fails if the death→effect bridge or the resolver is unwired).
//! - **`LeaveField` spawns the referenced field at the death location**: a cover tile with a
//!   `LeaveField` on-death effect that is destroyed by fire spawns the referenced field at that
//!   cell (the second headline test), persisting per GTW-545 rules.
//! - **Both variants are RON-authorable on weapons AND cover tiles**: the weapon spec's
//!   `on_death` field + the terrain def's `on_death` field carry the effects through setup.
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / field-present — never a
//! specific number. HARNESS NOTE (the `aoe_hittype` / `melee_act` / `melee_cover_smash`
//! idiom): the sim crate is the LOW crate,
//! so it drives `setup_battle_on_request` via `SetupBattleRequested` against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app (the EXACT production wiring).

mod explode_fans;
mod harness;
mod leave_field;
