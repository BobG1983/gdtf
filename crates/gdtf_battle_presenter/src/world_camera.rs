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

use bevy::{camera::visibility::RenderLayers, prelude::*};

/// The non-zero render layer the world camera renders.
///
/// Framework plumbing — the inner `Layer` (`usize`) index fed to
/// [`RenderLayers::layer`], an index into a collection Bevy owns, NOT a domain value
/// (the no-bare-types framework-plumbing carve-out, mirroring the `CELL_PX`-is-a-const
/// reasoning). It MUST be non-zero so the world camera's `RenderLayers` does not
/// intersect layer 0 — the default layer the GTW-120 UI camera falls to (a plain
/// `Camera2d` with no explicit `RenderLayers` resolves to `RenderLayers::layer(0)`).
pub const WORLD_RENDER_LAYER: usize = 1;

/// The render order of the world camera — lower than the UI camera so it renders first.
///
/// Framework plumbing — the `isize` written into `Camera.order` (whose default is `0`).
/// Negative so the world camera renders BENEATH the GTW-120 UI camera, which is a plain
/// `Camera2d` at the default order `0`.
const WORLD_CAMERA_ORDER: isize = -1;

/// Marker for the single SHARED world [`Camera2d`] owned by `BattlePresenterPlugin`.
///
/// This is plumbing around the framework camera (the `Camera2d` itself is exempt from
/// the no-bare-types rule, the same justification `UiCamera` uses), not a domain value.
/// It is `pub` so the S7 input crate (`gdtf_battle_input`) can query `With<WorldCamera>`
/// to read the world camera, and so the app can despawn precisely *this* camera on exit
/// without re-querying every `Camera2d` in the world.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WorldCamera;

/// Spawns the single SHARED world [`Camera2d`] (the [`WorldCamera`]).
///
/// Registered by the app on `OnEnter(GameState::BattleScape)` (the presenter cannot name
/// `GameState`). It spawns a `Camera2d` carrying the [`WorldCamera`] marker, a
/// [`Camera`] at [`WORLD_CAMERA_ORDER`] (`-1`, below the UI camera's default `0` so it
/// renders first / beneath), and [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so
/// it renders its own non-zero layer and does not intersect the UI camera's layer 0.
///
/// Param-only (`Commands`): spawning is `Commands::spawn`, never `&mut World`
/// (`bevy-traps.md` #7).
pub fn spawn_world_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        WorldCamera,
        Camera {
            order: WORLD_CAMERA_ORDER,
            ..default()
        },
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

/// Despawns every [`WorldCamera`] entity.
///
/// Registered by the app on `OnExit(GameState::BattleScape)` — the same span as the
/// existing `request_battle_teardown`, so the camera survives the whole
/// `Generation → AnimateIn → BattleRunning → AnimateOut → AfterMath` walk and is torn
/// down only when the battle is left. Tolerates zero or one `WorldCamera` (it despawns
/// whatever the `With<WorldCamera>` query yields).
///
/// Param-only (`Commands` + a `Query<Entity, With<WorldCamera>>`): despawning is
/// `Commands::entity(..).despawn()`, never `&mut World` (`bevy-traps.md` #7).
pub fn despawn_world_camera(mut commands: Commands, cameras: Query<Entity, With<WorldCamera>>) {
    for camera in &cameras {
        commands.entity(camera).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The configured world render layer does not intersect the UI camera's default
    /// layer 0 (the local guarantee behind AC3) — exercised through the real
    /// `RenderLayers::intersects` so it is genuine runtime behaviour, not a constant.
    #[test]
    fn world_render_layer_misses_layer_zero() {
        let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
        let ui_layer = RenderLayers::layer(0);
        assert!(
            !world_layer.intersects(&ui_layer),
            "the world camera's render layer ({WORLD_RENDER_LAYER}) must not intersect the UI \
             camera's default layer 0",
        );
    }
}
