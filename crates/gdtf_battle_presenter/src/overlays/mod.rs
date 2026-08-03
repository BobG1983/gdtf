//! HUD overlays drawn over the battlefield.

/// Badges for threats and connectors on other storeys.
pub mod cross_level_signals;
/// Persistent field coverage tiles.
pub mod field;
/// Fire-mode target cell highlight.
pub mod fire_target;
/// Hover / squad-visible cell highlight.
pub mod highlight;
/// Planned move path steps and cost label.
pub mod path_preview;
/// Shared sprite-pool grow/hide helper.
pub mod pool;
/// The reachable-range DEBUG overlay (GTW-450) — render-only, so the whole module
/// compiles ONLY in a debug build (`#[cfg(debug_assertions)]`, C1). In a release
#[cfg(debug_assertions)]
pub mod reachable;
/// Whether a cell is squad-visible for targeting UI.
pub mod targeting_gate;
