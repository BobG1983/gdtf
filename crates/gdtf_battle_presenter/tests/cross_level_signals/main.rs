//! GTW-596 cross-level tactical badges — headless integration proof that the
//! REGISTERED `derive_cross_level_signals` + `draw_cross_level_signals` systems
//! wire the real sim resources/queries into `CrossLevelSignals` and its pooled
//! badge sprites/labels (the `vertical_link_draw` / `reachable_overlay.rs`
//! pattern). The sibling pure-logic tests (`overlays/cross_level_signals/test/`)
//! pin the gather/aggregate/cap logic WITHOUT an app.

mod active_level_redraw;
mod cap;
mod connector;
mod drop_depth;
mod harness;
mod threat;
