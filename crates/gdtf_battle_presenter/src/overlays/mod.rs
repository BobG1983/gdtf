pub mod cross_level_signals;
pub mod field;
pub mod fire_target;
pub mod highlight;
pub mod path_preview;
pub mod pool;
/// The reachable-range DEBUG overlay (GTW-450) — render-only, so the whole module
/// compiles ONLY in a debug build (`#[cfg(debug_assertions)]`, C1). In a release
#[cfg(debug_assertions)]
pub mod reachable;
pub mod targeting_gate;
