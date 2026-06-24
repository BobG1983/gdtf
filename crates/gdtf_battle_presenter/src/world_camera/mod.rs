//! The SHARED (renderer-independent) world-camera lifecycle for the GTW-48 battle
//! presenter.
//!
//! A single `Camera2d` carrying the [`WorldCamera`] marker is spawned on entry to the
//! battle and despawned on exit. It is configured to render BENEATH the persistent
//! GTW-120 UI camera (a lower `Camera.order`) and on its own non-zero render layer
//! ([`WORLD_RENDER_LAYER`]) so it does not composite onto the UI camera's default
//! layer-0 surface.
//!
//! This camera is the surface S3+ will draw glyph sprites against and the camera S7's
//! input crate (`gdtf_battle_input`) will query (via `With<WorldCamera>`) to unproject
//! the cursor. It deliberately draws no glyph, loads no atlas, and reads no sim
//! state — it is owned by the SHARED presenter layer (`BattlePresenterPlugin`), NOT a
//! specific renderer, so a later iso renderer swap reuses it unchanged.
//!
//! The spawn/despawn systems are `pub` and param-only (`Commands` / `Query`): the app
//! registers them on the `GameState::BattleScape` boundary because the presenter has no
//! `gdtf_app` dependency and so cannot name `GameState`. The crate dependency chain
//! (input -> presenter -> sim, one-way; see `docs/decisions/0001-rust-bevy-rewrite.md`
//! and `CLAUDE.md`) puts `WorldCamera` in the presenter so the input crate can depend
//! on it.
//!
//! The lifecycle splits across five concern modules: [`mod@marker`] (the marker +
//! spawn/despawn), [`mod@framing`] (the GTW-249 battle-start frame-on-units + the
//! bounds clamp), [`mod@pan`] (the GTW-250 pan-navigation helpers/systems + the
//! GTW-259 gamepad-cursor edge-pan message), [`mod@tuning`] (the GTW-299
//! hot-reloadable edge-pan tuning table + its RON load / resolve / re-derive chain), and
//! [`mod@dwell`] (the GTW-299 edge-band dwell accumulator + the pure dwell decision).

mod dwell;
mod framing;
mod marker;
mod pan;
mod tuning;

#[cfg(test)]
mod test;

pub use dwell::{DwellElapsed, PanEdgeDwellState, should_edge_pan_after_dwell};
pub use framing::{camera_focus, clamp_camera, clamp_camera_to_bounds, frame_camera_on_units};
pub use marker::{WORLD_RENDER_LAYER, WorldCamera, despawn_world_camera, spawn_world_camera};
pub use pan::{
    EdgeBandPx, GamepadCursorMoved, PanSpeed, STICK_DEADZONE, StickDeadzone, keyboard_pan_dir,
    mouse_edge_dir, pan_camera, pan_camera_on_gamepad_cursor_edge, pan_velocity, stick_pan_dir,
    viewport_edge_dir,
};
pub use tuning::{
    BoundsMarginWorld, DwellDelaySeconds, PanTuning, PanTuningHandle, load_pan_tuning,
    redrive_pan_tuning_on_asset_event, resolve_pan_tuning,
};
