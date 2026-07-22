//! The world-camera marker, its render-layer / order consts, and the spawn/despawn
//! lifecycle systems.

use bevy::{
    camera::{ClearColorConfig, visibility::RenderLayers},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};

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

/// The background color the world camera clears the whole render surface to (GTW-271).
///
/// Framework plumbing — a bare [`Color`] written into the world camera's
/// [`Camera::clear_color`] (a knob on a framework component, the same carve-out as
/// [`WORLD_CAMERA_ORDER`] / [`WORLD_CAMERA_SCALE`]). The world camera renders FIRST (order
/// [`WORLD_CAMERA_ORDER`] `= -1`) and `LoadOp::Clear` clears the ENTIRE surface (it is NOT
/// scissored to the viewport sub-rect), so this color fills the MARGINS around the map
/// sub-rect; the map then blits scissored into the [`Camera::viewport`] region the app sets
/// (GTW-271 AC1), and the UI camera composites OVER without clearing (its
/// [`ClearColorConfig::None`] in the app's `spawn_ui_camera`). A dark grimdark slate so the
/// margins read as inert frame, not battlefield.
const MARGIN_BG: Color = Color::srgb(0.06, 0.06, 0.08);

/// The world camera's orthographic projection `scale` (GTW-263 — battle zoom).
///
/// Framework plumbing — the bare `f32` written into
/// [`OrthographicProjection::scale`], a knob on a framework component (not a domain
/// value), so it is a documented const here (the same carve-out as [`WORLD_CAMERA_ORDER`]
/// / [`WORLD_RENDER_LAYER`]). `0.5` HALVES the visible world half-extent (the framing
/// reads `window * 0.5 * scale`), so the battlefield is drawn at TWICE the on-screen size
/// versus the `default_2d` `scale = 1.0` — the play-test "too zoomed out" fix. The GTW-249
/// `frame_camera_on_units` writes only the camera TRANSLATION, never the projection, so
/// this scale persists across framing / pan.
const WORLD_CAMERA_SCALE: f32 = 0.5;

/// Marker for the single SHARED world [`Camera2d`] owned by `BattlePresenterPlugin`.
///
/// This is plumbing around the framework camera (the `Camera2d` itself is exempt from
/// the no-bare-types rule, the same justification `UiCamera` uses), not a domain value.
/// It is `pub` so the S7 input crate (`gdtf_battle_input`) can query `With<WorldCamera>`
/// to read the world camera, and so the app can despawn precisely *this* camera on exit
/// without re-querying every `Camera2d` in the world.
///
/// The derived [`Default`] is a **spawn-seed sentinel only** (GTW-322): a fieldless
/// marker, so its `Default` is the same zero-sized value, present purely so the
/// reflection-free `bsn!` macro can seed the component slot. It carries no state.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub struct WorldCamera;

/// Spawns the single SHARED world [`Camera2d`] (the [`WorldCamera`]).
///
/// Registered by the app on `OnEnter(GameState::BattleScape)` (the presenter cannot name
/// `GameState`). It spawns a `Camera2d` carrying the [`WorldCamera`] marker, a
/// [`Camera`] at `WORLD_CAMERA_ORDER` (`-1`, below the UI camera's default `0` so it
/// renders first / beneath) whose [`Camera::clear_color`] is the GTW-271 margin bg
/// ([`ClearColorConfig::Custom`]`(MARGIN_BG)` — fills the margins around the map sub-rect),
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so
/// it renders its own non-zero layer and does not intersect the UI camera's layer 0, and
/// a [`Projection::Orthographic`] at `WORLD_CAMERA_SCALE` (`0.5` — 2x zoom, GTW-263)
/// OVERRIDING the `default_2d` `scale = 1.0` that `Camera2d` would otherwise
/// `#[require]`.
///
/// Param-only (`Commands`): spawning is `Commands::spawn`, never `&mut World`
/// (`bevy-traps.md` #7).
pub fn spawn_world_camera(mut commands: Commands) {
    // GTW-271 — clear the WHOLE surface to the margin bg (LoadOp::Clear is not
    // scissored to the viewport), so the area OUTSIDE the world-map viewport sub-rect
    // (the app sets that rect in AC1) renders a defined color rather than garbage; the
    // map blits scissored into the sub-rect and the UI camera composites over (its
    // `ClearColorConfig::None` in `spawn_ui_camera`).
    let camera = Camera {
        order: WORLD_CAMERA_ORDER,
        clear_color: ClearColorConfig::Custom(MARGIN_BG),
        ..default()
    };
    let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
    // GTW-263 — zoom the battle in 2x. `Camera2d` `#[require]`s a default-2d
    // orthographic projection at `scale = 1.0`; spawn the projection explicitly with
    // `scale = 0.5` so the visible half-extent halves (the framing reads `window * 0.5
    // * scale`), drawing the battlefield at twice the on-screen size. The rest of the
    // projection keeps the `default_2d` values (near plane, scaling mode, etc.).
    let projection = Projection::Orthographic(OrthographicProjection {
        scale: WORLD_CAMERA_SCALE,
        ..OrthographicProjection::default_2d()
    });
    // GTW-322 — authored as a `bsn!` scene. The fieldless framework / marker
    // components (`Camera2d`, `WorldCamera`) ride the macro directly (each has
    // `Default`); the runtime-valued `Camera`, `RenderLayers`, and `Projection`
    // have no `bsn!` value grammar, so they are each composed onto the SAME entity
    // with `template_value` (a `Clone + Default + Unpin` value-overwrite) — the
    // same entity + components result, only the spawn SHAPE changed.
    commands.spawn_scene((
        bsn! { Camera2d WorldCamera },
        template_value(camera),
        template_value(layers),
        template_value(projection),
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
