//! GTW-251 AC3: headless END-TO-END parity test for the message-driven hover-highlight.
//!
//! This wires the WHOLE chain in one app — the input `GdtfBattleInputPlugin` (the
//! `pick_hovered_cell` picker + the `emit_highlight_request` emitter) AND the presenter
//! `TopDownRendererPlugin` (the `draw_highlight_on_request` draw) — and proves the
//! refactor preserved behavior: with a hovered cell (the REAL picker path), the one
//! `HoverHighlight` sprite ends up Visible at `cell_to_world(hovered cell)` — the same
//! observable result the old input-side `update_hover_highlight` produced. It then
//! pin-discriminates: a wrong cell or no-emit fails, and an off-grid cursor hides it.
//!
//! The highlight is a solid-tint sprite (no atlas), so no `AssetServer` is needed: a
//! `MinimalPlugins` app + both plugins + a synthesized `WorldCamera`/`Window`/cursor +
//! `BattleInProgress` is sufficient. The camera/cursor/world mutations are in the TEST
//! BODY — the accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No function
//! here takes `&mut World`/`&World`.

use bevy::{
    app::App,
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    math::{Vec2, Vec3},
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{GdtfBattleInputPlugin, HoveredCell, world_to_cell};
use gdtf_battle_presenter::{
    ActiveLevel, HoverHighlight, TopDownRendererPlugin, WorldCamera, cell_to_world,
};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level};

/// The synthetic window/camera render-target size (physical px), large enough that a
/// cursor near its centre unprojects to an in-grid cell.
const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

/// Builds a deterministic `Camera` whose `viewport_to_world_2d` succeeds WITHOUT a
/// render pipeline, mirroring bevy's own `viewport_to_world` unit test (the
/// `picking.rs` recipe): set the render-target info + viewport, run the projection's
/// `update`, and store its clip-from-view matrix in `computed`.
fn synthetic_camera() -> Camera {
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(TARGET_SIZE.x, TARGET_SIZE.y);
    Camera {
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: TARGET_SIZE.as_uvec2(),
                scale_factor:  1.0,
            }),
            clip_from_view: projection.get_clip_from_view(),
            ..ComputedCameraValues::default()
        },
        ..Camera::default()
    }
}

/// Builds the full headless chain: `MinimalPlugins`, the input plugin (picker plus
/// emit), the presenter renderer (draw), a synthesized `WorldCamera`/`Window`, and the
/// `BattleInProgress` gate. The cursor is left unset until a test sets it.
fn e2e_app(active_level: Level) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(TopDownRendererPlugin);
    // TopDownRendererPlugin init_resource-s ActiveLevel(0) on build; override to the
    // level under test so the picker bands on it.
    app.world_mut().insert_resource(ActiveLevel(active_level));
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        synthetic_camera(),
        GlobalTransform::IDENTITY,
        Projection::Orthographic(OrthographicProjection::default_2d()),
        Frustum::default(),
    ));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

/// Sets the primary window's cursor position (or clears it).
fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// The current `HoveredCell` value.
fn hovered(app: &App) -> Option<CellLevel> {
    app.world().get_resource::<HoveredCell>().and_then(|h| **h)
}

/// The single hover-highlight sprite's translation + visibility, if exactly one exists.
fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<HoverHighlight>>();
    let mut iter = q.iter(app.world());
    iter.next().map(|(t, v)| (t.translation, *v))
}

/// Counts the hover-highlight sprites in the world.
fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    q.iter(app.world()).count()
}

/// AC3 — END-TO-END: the real picker resolves the hovered cell, the emitter writes the
/// request, the presenter draws — the one highlight ends up Visible at
/// `cell_to_world(hovered)`. Two `update()`s let the emit (input band) and the draw
/// (presenter band, `.after(SimSystems::Simulate)`) both observe the same cursor across
/// frames. Pin-discriminating: it must be at the EXACT hovered cell (a wrong cell
/// fails), and an off-grid cursor hides it.
#[test]
fn end_to_end_highlight_lands_at_the_hovered_cell() {
    let level = Level::new(0);
    let mut app = e2e_app(level);

    // An in-grid cursor (shifted right + down — see picking.rs for the screen->world
    // sign reasoning).
    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    // Frame 1: picker resolves HoveredCell + emits; the draw reads the same frame's
    // message (presenter band runs after the input band). A second update settles any
    // ordering across the input/presenter bands deterministically.
    app.update();
    app.update();

    let cell = hovered(&app);
    assert!(cell.is_some(), "the in-grid cursor must resolve a cell");
    let Some(cell) = cell else { return };

    // The highlight is Visible at EXACTLY cell_to_world(hovered) — the same observable
    // result the old input-side draw produced.
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one highlight sprite after the real picker path",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((
            cell_to_world(Cell::new(cell.x, cell.y), level),
            Visibility::Visible,
        )),
        "the highlight must be Visible at cell_to_world(the hovered cell)",
    );

    // Pin-discriminate the cell: a DIFFERENT cell's world position must NOT match (so a
    // wrong-cell draw would fail this test).
    let wrong = cell_to_world(Cell::new(cell.x + 3, cell.y + 3), level);
    assert_ne!(
        highlight_state(&mut app).map(|(t, _)| t),
        Some(wrong),
        "the highlight must be at the hovered cell, not a neighbouring one",
    );

    // Off-grid cursor: HoveredCell -> None, the request -> None, the highlight hides.
    let cursor_off = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
    set_cursor(&mut app, Some(cursor_off));
    // Confirm the chosen cursor really is off-grid via the documented inverse.
    let world_off = {
        let mut q = app
            .world_mut()
            .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
        q.iter(app.world())
            .next()
            .and_then(|(cam, t)| cam.viewport_to_world_2d(t, cursor_off).ok())
    };
    let Some(world_off) = world_off else { return };
    assert_eq!(
        world_to_cell(world_off, level),
        None,
        "the chosen left-of-origin cursor must be off-grid (world {world_off:?})",
    );
    app.update();
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "the off-grid cursor clears HoveredCell"
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "the highlight entity persists (hidden, not duplicated) off-grid",
    );
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Hidden),
        "the highlight must hide when the cursor leaves the grid",
    );
}
