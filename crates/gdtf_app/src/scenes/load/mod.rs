mod plugin;
mod systems;
pub(in crate::scenes) use plugin::LoadScenePlugin;
mod resources;
// The resolved authored battlefield resource the Generation slice (E10.5) reads;
// it persists past `OnExit(Load)`, so it is the load scene's outward-facing
// product (`LoadedSituation` is `pub(crate)` at its definition via `support_item!`,
// reachable in-crate as `crate::scenes::load::resources::LoadedSituation`).
//
// This re-export feeds the test-support surface (the AC7 real-asset harness names
// `crate::scenes::LoadedSituation`); it is test-support-gated so the binary build
// stays `unused`-clean until E10.5 consumes it in-crate. (GTW-205)
#[cfg(feature = "test-support")]
crate::support_use!(resources::LoadedSituation;);
