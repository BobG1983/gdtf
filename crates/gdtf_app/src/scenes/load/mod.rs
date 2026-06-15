mod plugin;
mod systems;
pub(in crate::scenes) use plugin::LoadScenePlugin;
mod resources;
// The resolved authored battlefield resource the Generation slice (E10.5) reads;
// it persists past `OnExit(Load)`, so it is the load scene's outward-facing
// product (`LoadedSituation` is `pub(crate)` at its definition via `support_item!`).
//
// Re-exported here so the in-crate path `crate::scenes::load::LoadedSituation` is
// nameable (the `resources` module is private). E10.5's `request_battle_setup`
// consumes it in-crate in the binary build (`pub(crate)`), and the `scenes/mod.rs`
// re-export widens it to `pub` under `test-support` for the integration harness
// (`crate::scenes::LoadedSituation`) — so the visibility tracks `support_item` in
// lockstep (the re-export chain caveat in `support_use`). (GTW-205 / GTW-207)
crate::support_use!(resources::LoadedSituation;);
