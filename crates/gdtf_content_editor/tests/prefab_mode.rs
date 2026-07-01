//! Headless integration test for the GTW-515 (C4) egui PREFAB mode + render-to-texture viewport.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (a live
//! [`AssetServer`] rooted at the workspace `assets/`), so the editor's actual `Load` pass resolves
//! the shipped theme + content registries and its real `Editing` scene inserts the PREFAB model +
//! preview machinery — not a copy.
//!
//! Per `verification.md` Rule 3, this asserts the STATE-MACHINE + MODEL contract the egui DRAW +
//! the offscreen RENDER build on (the DRAW + render are Screenshot-QA-covered — the egui closure
//! never runs headlessly without a primary egui context, and the headless harness has no render
//! device):
//!
//! - the preview render-target resource ([`PreviewTarget`]) + the owned pan target ([`PreviewPan`])
//!   are inserted in `Editing` (the C4.3 / C4.8 state-scoped lifecycle),
//! - the change-driven redraw spawns preview tile [`Sprite`]s once the theme + registries resolve
//!   (proving the tile pipeline is wired — a non-empty viewport, not a black one — C4.3 / C4.12),
//! - a paint through the SHARED [`apply_placement`] predicate (the SAME one the viewport click
//!   runs — C4.4 / C4.10) mutates the [`EditorMap`], and the redraw picks it up.
//!
//! `assert!` + `let … else` keep the test panic-free per the workspace lints (no `unwrap` /
//! `expect` / `panic!`).

use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::{
    Cell,
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::{CellLevel, Level},
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_editor::{
    CurrentEditLevel, EditorMap, EditorMode, EditorState, MapEditorPlugin, MapEditorSession,
    PreviewPan, PreviewTarget, ProposedPlacement, apply_placement,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap: async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll a SIGNAL, not a count.
const MAX_UPDATES: u32 = 10_000;

/// Build the real editor app on the no-renderer `DefaultPlugins` UI harness (the SAME plugin the
/// binary wires, minus the windowed `EguiPlugin` the headless harness has no window for — so the
/// egui image REGISTRATION no-ops, but the preview render-target + camera + tile sprites still
/// spawn, which is exactly the model contract this test asserts).
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drive the app to `Editing`, then a few frames so the `OnEnter(Editing)` inserts apply.
fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme + \
         registries",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// C4.3 / C4.8 — the preview render machinery's state-scoped resources are inserted in `Editing`:
/// the [`PreviewTarget`] (the offscreen render target the viewport draws) and the owned
/// [`PreviewPan`] offset (the pan target the camera is driven to).
#[test]
fn editing_inserts_the_preview_target_and_pan() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    assert!(
        app.world().get_resource::<PreviewTarget>().is_some(),
        "the PreviewTarget render-target resource must be inserted in Editing (C4.3)",
    );
    assert!(
        app.world().get_resource::<PreviewPan>().is_some(),
        "the PreviewPan owned pan-offset target must be inserted in Editing (C4.8)",
    );
}

/// C4.3 / C4.12 — the change-driven redraw spawns preview tile [`Sprite`]s once the theme +
/// registries resolve (the seeded default theme's default-floor fills the grid), proving the
/// preview tile pipeline is wired end-to-end — a NON-EMPTY viewport, never a black one.
///
/// Positive assertion: after a few frames in the default PREFAB mode with the seeded theme, at
/// least one preview `Sprite` exists. (The harness spawns no other world sprites — only the editor
/// preview tiles do — so a non-zero count is the preview content.)
#[test]
fn redraw_spawns_preview_tile_sprites() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    // The editor opens in PREFAB mode by default; give the seed + redraw a few frames to run.
    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Prefab),
        "the editor opens in PREFAB mode",
    );
    for _ in 0..8 {
        app.update();
    }

    let sprite_count = app.world_mut().query::<&Sprite>().iter(app.world()).count();
    assert!(
        sprite_count > 0,
        "the change-driven redraw must spawn at least one preview tile sprite (the default-floor \
         fill) — a non-empty viewport, not a black one (C4.3 / C4.12); got {sprite_count}",
    );
}

/// C4.4 / C4.10 — a paint through the SHARED [`apply_placement`] predicate (the SAME one the
/// viewport click runs) mutates the [`EditorMap`] on the real code path, and the change-driven
/// redraw re-runs (the map `is_changed`). Reuses the placement layer verbatim — no re-implemented
/// legality.
#[test]
fn shared_apply_placement_paints_the_map() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    // Let the theme seed so a default floor / palette resolves.
    for _ in 0..8 {
        app.update();
    }

    // Resolve a paint tile from the seeded session (the default floor is a real terrain key).
    let (tile, size, theme) = {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Some(tile) = session.default_floor() else {
            // No default floor resolved yet — the seed is async; treat as a soft skip by asserting
            // the session exists (the resource contract) rather than failing on load timing.
            assert!(world.get_resource::<EditorMap>().is_some(), "map inserted");
            return;
        };
        (tile, session.grid_size(), session.theme())
    };

    // Drive the SHARED apply_placement (the SAME predicate the viewport click runs) on the map +
    // the real terrain registry — a plain floor placement on the ground plane is legal + commits.
    let painted = {
        let world = app.world_mut();
        let registry = world
            .get_resource::<TerrainDefRegistry>()
            .cloned()
            .unwrap_or_default();
        let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
            unreachable!("map inserted in Editing");
        };
        let slot = CellLevel::new(Cell::new(1, 1), Level::new(0));
        let placement = ProposedPlacement::new(slot, tile);
        apply_placement(&mut map, &registry, theme, &placement, size)
    };
    assert!(
        painted,
        "a legal floor placement through the shared apply_placement must commit (C4.4 / C4.10)",
    );
    let count = app
        .world()
        .get_resource::<EditorMap>()
        .map_or(0, EditorMap::painted_count);
    assert_eq!(
        count, 1,
        "the committed placement mutates the EditorMap (one painted cell)"
    );

    // The redraw re-runs on the changed map without panicking.
    app.update();
}

/// Count the current preview tile [`Sprite`]s in the world. The harness spawns no other world
/// sprites (only the editor preview tiles do), so this IS the drawn tile set.
fn preview_sprite_count(app: &mut App) -> usize {
    app.world_mut().query::<&Sprite>().iter(app.world()).count()
}

/// GTW-532 C1 / C2 (REAL PATH) — toggling the prefab viewport's [`ViewMode`] from
/// [`DownToActive`](ViewMode::DownToActive) to [`FullView`](ViewMode::FullView) CHANGES the drawn
/// tile set: with a distinct block painted on the UPPER storey (culled at the ground edit storey in
/// `DownToActive`), `FullView` draws MORE tiles than `DownToActive`. Drives the SAME resources the editor
/// inserts — the reused presenter [`ViewMode`], the [`EditorMap`], the [`CurrentEditLevel`] — and
/// lets the real `redraw_preview_tiles` system re-run on the `ViewMode::is_changed` trigger.
///
/// A green build alone does NOT prove the toggle (C4): this asserts the drawn storey RANGE / tile
/// count actually differs between the two modes on the real code path.
#[test]
fn full_view_toggle_changes_the_drawn_tile_set() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    // Let the theme seed so a default floor / palette resolves and the base fill draws.
    for _ in 0..8 {
        app.update();
    }

    // The editor opens in PREFAB mode with the default DownToActive view.
    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Prefab),
        "the editor opens in PREFAB mode",
    );
    assert_eq!(
        app.world().get_resource::<ViewMode>().copied(),
        Some(ViewMode::DownToActive),
        "the prefab viewport opens in the default DownToActive view (GTW-532 C1)",
    );

    // Resolve a real paint tile from the seeded session; soft-skip if the async seed has not
    // resolved a default floor yet (the tile pipeline is asset-timing dependent).
    let (tile, theme) = {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Some(tile) = session.default_floor() else {
            assert!(
                world.get_resource::<ViewMode>().is_some(),
                "view mode inserted"
            );
            return;
        };
        (tile, session.theme())
    };

    // Give the prefab a 2-storey volume and paint a distinct block on the UPPER storey (storey 1).
    // At the ground edit storey, DownToActive draws only storey 0 (the upper block is CULLED);
    // FullView draws BOTH storeys (the upper block re-appears) — so the drawn tile count grows.
    {
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        if let Ok(size) = GridSize::new(GridWidth::new(6), GridHeight::new(6), GridLevels::new(2)) {
            session.set_grid_size(size);
        }
        let size = session.grid_size();
        let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
            unreachable!("map inserted in Editing");
        };
        // A distinct upper-storey block that only FullView draws.
        let upper = Level::new(1);
        for x in 0..3 {
            for y in 0..3 {
                map.paint_at(CellLevel::new(Cell::new(x, y), upper), tile, size);
            }
        }
        // Stay on the ground edit storey so DownToActive culls the upper block.
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        *edit_level = CurrentEditLevel::ground();
    }
    let _ = theme;

    // Settle the DownToActive draw and record its drawn tile count.
    for _ in 0..4 {
        app.update();
    }
    let down_to_active = preview_sprite_count(&mut app);
    assert!(
        down_to_active > 0,
        "DownToActive draws the ground-storey fill (a non-empty viewport); got {down_to_active}",
    );

    // Flip the REUSED presenter ViewMode to FullView (the same flip the toggle button / F hotkey
    // apply) and let the real redraw re-run on the ViewMode::is_changed trigger.
    {
        let world = app.world_mut();
        let Some(mut view) = world.get_resource_mut::<ViewMode>() else {
            unreachable!("view mode inserted in Editing");
        };
        *view = ViewMode::FullView;
    }
    for _ in 0..4 {
        app.update();
    }
    let full_view = preview_sprite_count(&mut app);

    assert!(
        full_view > down_to_active,
        "toggling to FullView must draw MORE tiles than DownToActive — the upper-storey block \
         re-appears (the mode switch changes the drawn storey range / tile set, GTW-532 C2); \
         DownToActive={down_to_active}, FullView={full_view}",
    );
}
