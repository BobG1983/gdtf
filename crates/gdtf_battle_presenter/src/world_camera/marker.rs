//! The world-camera marker, its render-layer / order consts, and the spawn/despawn
//! lifecycle systems.

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
