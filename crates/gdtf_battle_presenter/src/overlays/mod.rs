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
/// Reachable-range debug overlay, render-only. Its draw system registers in debug builds only.
pub mod reachable;
/// Whether a cell is squad-visible for targeting UI.
pub mod targeting_gate;
