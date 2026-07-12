//! Pure-logic unit tests for the GTW-596 cross-level signals gather pipeline —
//! the fog-gating invariant (both directions, on the real `is_ganger_visible` /
//! `is_cell_explored` path), the per-producer positive assertions, and the
//! aggregation + cap regression cases (`cap.rs`). The headless integration proof
//! that the REGISTERED derive + draw systems wire these through live Bevy
//! resources/queries lives in `crates/gdtf_battle_presenter/tests/cross_level_signals/`
//! (the `vertical_link_draw` / `reachable_overlay.rs` pattern).

mod cap;
mod connector;
mod drop_depth;
mod threat;
