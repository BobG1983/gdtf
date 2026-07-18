mod plugin;
mod systems;
// GTW-582: re-export the cfg(test) tracing-capture scaffold one hop up so
// sibling scene modules (`crate::states::load::hot_reload_test_support`) can
// drive log-capture assertions through the ONE shared, poison-proof path.
pub(in crate::states) use plugin::LoadScenePlugin;
#[cfg(test)]
pub(crate) use systems::hot_reload_test_support;
mod fallbacks;
// The bespoke headless Load-fallback seed (GTW-629) — Load-orchestration policy
// owned here. TEST-ONLY (GTW-749: its only production caller, the dev-only
// auto-battle affordance, was retired outright): `pub` under `test-support`
// only, so the re-export widens in lockstep with the item's own gate — never a
// `pub(crate)` production path.
#[cfg(feature = "test-support")]
pub use fallbacks::seed_load_fallbacks;
mod resources;
// The resolved authored battlefield resource the Generation slice (E10.5) reads;
// it persists past `OnExit(Load)`, so it is the load scene's outward-facing
// product (`LoadedSituation` is `pub(crate)` at its definition via `support_item!`).
//
// Re-exported here so the in-crate path `crate::states::load::LoadedSituation` is
// nameable (the `resources` module is private). E10.5's `request_battle_setup`
// consumes it in-crate in the binary build (`pub(crate)`), and the `states/mod.rs`
// re-export widens it to `pub` under `test-support` for the integration harness
// (`crate::states::LoadedSituation`) — so the visibility tracks `support_item` in
// lockstep (the re-export chain caveat in `support_use`). (GTW-205 / GTW-207)
crate::support_use!(resources::LoadedSituation;);
