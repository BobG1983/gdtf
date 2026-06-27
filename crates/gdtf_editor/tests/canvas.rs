//! Headless integration test for the GTW-423 map-editor central canvas.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the same
//! harness the GTW-417 `editor_shell` / GTW-421 `right_panel` / GTW-422 `palette` tests use), so
//! the editor's actual `Load` pass resolves the shipped theme + catalog registries and its real
//! `Editing` scene builds the canvas — not a copy.
//!
//! Asserts the contract end-to-end, pin-discriminatingly, and value-agnostically (counts /
//! extent DERIVED from `grid_size`, never a pinned px magnitude):
//!
//! - T1 (C1): after a size is set, a boundary element (the [`CanvasRoot`]) exists carrying a
//!   [`CanvasExtent`] whose width/height equal the session's `grid_size` x/y spans.
//! - T2 (C2): the per-cell dimmed dashes exist — every [`CanvasCell`] carries a `BorderColor`
//!   (the lightweight per-cell grid-line mechanism), one per cell.
//! - T3 (C3): the canvas has `width × height` [`CanvasCell`] fills, each carrying the
//!   default-floor tile sprite ([`ImageNode`]) at the catalog's default-floor atlas index.
//! - T4 (C4): committing a SMALLER `grid_size` re-extents the canvas (new cell count == the new
//!   `width × height`); switching the THEME re-fills the cells with the new theme's default-floor
//!   atlas index.
//! - T5 (scroll-parenting guard, mirrors GTW-421/422): a [`ScrollListArea`] is an ANCESTOR of the
//!   [`CanvasRoot`] (the grid hangs inside the clipping, scrolling viewport, not the grid frame).
//!
//! Panic/expect-free per the workspace lints (the GTW-417 precedent: `assert!` + `if let Some`
//! guards, never `unwrap`/`expect`/`panic`).

use bevy::{
    prelude::*,
    ui::{BorderColor, widget::ImageNode},
};
use gdtf_battle_sim::level::{GridSize, LevelTheme, ThemeCatalogRegistry, TileAtlasIndex};
use gdtf_editor::{
    CanvasCell, CanvasExtent, CanvasRoot, EditorState, GridSpanInput, MapEditorPlugin,
    MapEditorSession, SizeFieldAxis, ThemeDropdown,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{
    CommittedNumericValue, DropdownSelectionChanged, NumericFieldCommitted, ScrollListArea,
    UiPlugin,
};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness and advances it to
/// [`EditorState::Editing`] with the canvas built across the deferred-parent frames.
fn editor_in_editing() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    app.add_plugins(MapEditorPlugin);

    let reached = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme \
         + registries (a genuine load failure, not a frame-budget shortfall)",
    );
    // The OnEnter spawn + the Update `sync_canvas` build the cells across a few frames (deferred
    // re-parent commands); advance so the canvas is present before we query.
    settle(&mut app);
    app
}

/// Advance a handful of frames so the deferred scroll-list re-parent + the `sync_canvas` rebuild
/// + its deferred grid re-parent have all applied.
fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// Count the entities carrying the marker `M`.
fn count<M: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&M>().iter(app.world()).count()
}

/// The session's current `grid_size`, if present.
fn grid_size(app: &App) -> Option<GridSize> {
    app.world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::grid_size)
}

/// The active theme's catalog's declared default-floor atlas index — the index every canvas
/// cell must fill with (C3).
fn default_floor_index(app: &App, theme: LevelTheme) -> Option<TileAtlasIndex> {
    app.world()
        .get_resource::<ThemeCatalogRegistry>()
        .and_then(|reg| reg.catalog(theme))
        .and_then(|cat| cat.default_floor())
        .map(|tile| tile.atlas_index)
}

/// The single [`CanvasRoot`]'s [`CanvasExtent`], if exactly one root exists.
fn canvas_extent(app: &mut App) -> Option<CanvasExtent> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&CanvasExtent, With<CanvasRoot>>();
    q.iter(world).next().copied()
}

/// T1 (C1): once a size is set, a boundary element (the [`CanvasRoot`]) exists, carrying a
/// [`CanvasExtent`] sized to the session's `grid_size` x/y extent (NOT a pinned px magnitude).
#[test]
fn canvas_boundary_extent_tracks_grid_size() {
    let mut app = editor_in_editing();

    assert_eq!(
        count::<CanvasRoot>(&mut app),
        1,
        "exactly one canvas boundary (CanvasRoot) must exist once a size is set (C1)",
    );

    let size = grid_size(&app);
    assert!(size.is_some(), "the session must hold a grid size");
    let extent = canvas_extent(&mut app);
    assert!(
        extent.is_some(),
        "the canvas root must carry a CanvasExtent"
    );
    if let (Some(size), Some(extent)) = (size, extent) {
        assert_eq!(
            (extent.width(), extent.height()),
            (size.width(), size.height()),
            "the boundary extent must equal the session grid_size x/y spans (C1)",
        );
    }
}

/// T2 (C2): the per-cell dimmed dashes exist — every [`CanvasCell`] carries a `BorderColor` (the
/// lightweight per-cell grid-line mechanism), one per cell. Pin-discriminating: the dash count
/// equals the cell count (so the mechanism is the per-cell border, not an absent / shared one).
#[test]
fn per_cell_dashes_present_one_per_cell() {
    let mut app = editor_in_editing();

    let cells = count::<CanvasCell>(&mut app);
    assert!(cells > 0, "the canvas must have drawable cells");

    let dashed = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&BorderColor, With<CanvasCell>>();
        q.iter(world).count()
    };
    assert_eq!(
        dashed, cells,
        "every drawable cell must carry a per-cell dimmed-dash border (C2)",
    );
}

/// T3 (C3): the canvas has `width × height` [`CanvasCell`] fills, each carrying the default-floor
/// tile sprite ([`ImageNode`]) at the catalog's default-floor atlas index. Pin-discriminating:
/// the cell count equals the `grid_size` extent's cell count AND the fill sprite's atlas index is
/// the active theme's default-floor index (value-agnostic — derived from the catalog, not pinned).
#[test]
fn cells_prefilled_with_default_floor_sprite() {
    let mut app = editor_in_editing();

    let expected_count =
        grid_size(&app).map(|s| usize::from(*s.width()) * usize::from(*s.height()));
    let cell_count = count::<CanvasCell>(&mut app);
    assert_eq!(
        Some(cell_count),
        expected_count,
        "the canvas must hold one cell per grid_size width × height cell (C3)",
    );

    let theme = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    let expected_index = theme.and_then(|t| default_floor_index(&app, t));
    assert!(
        expected_index.is_some(),
        "the active theme's catalog must declare a resolvable default floor (C3)",
    );

    // Every cell's fill ImageNode must be the default-floor atlas index.
    let world = app.world_mut();
    let mut q = world.query_filtered::<&ImageNode, With<CanvasCell>>();
    let indices: Vec<Option<usize>> = q
        .iter(world)
        .map(|node| node.texture_atlas.as_ref().map(|atlas| atlas.index))
        .collect();
    assert!(!indices.is_empty(), "there must be filled cells to check");
    if let Some(expected) = expected_index {
        for idx in indices {
            assert_eq!(
                idx,
                Some(*expected),
                "every cell must be pre-filled with the default-floor atlas index (C3)",
            );
        }
    }
}

/// T4a (C4): committing a SMALLER `grid_size` re-extents the canvas — the new cell count equals
/// the new `width × height`, and the boundary extent tracks it. Drives the REAL size-field commit
/// message the GTW-421 width/height fields emit.
#[test]
fn shrinking_grid_size_reextents_canvas() {
    let mut app = editor_in_editing();

    let start_cells = count::<CanvasCell>(&mut app);
    assert!(start_cells > 0, "the canvas must start populated");

    // Commit width = 8, height = 8 via the real size-field commit messages, routed by axis. The
    // editor's size fields carry SizeFieldAxis markers; find one per axis and emit a commit for it.
    let (width_field, height_field) = size_fields(&mut app);
    assert!(
        width_field.is_some() && height_field.is_some(),
        "the editor must have width + height size fields to drive (C4)",
    );
    if let (Some(width_field), Some(height_field)) = (width_field, height_field) {
        app.world_mut().write_message(NumericFieldCommitted::new(
            width_field,
            CommittedNumericValue::new(GridSpanInput::new(8)),
        ));
        app.world_mut().write_message(NumericFieldCommitted::new(
            height_field,
            CommittedNumericValue::new(GridSpanInput::new(8)),
        ));
    }
    settle(&mut app);

    // The session shrank to 8 × 8 (× the unchanged levels) and the canvas re-extented to 64 cells.
    let size = grid_size(&app);
    if let Some(size) = size {
        assert_eq!(
            *size.width(),
            8,
            "the width commit must shrink the grid (C4)"
        );
        assert_eq!(
            *size.height(),
            8,
            "the height commit must shrink the grid (C4)"
        );
    }
    let new_cells = count::<CanvasCell>(&mut app);
    assert_eq!(
        new_cells, 64,
        "shrinking to 8 × 8 must re-extent the canvas to 64 cells (C4)",
    );
    let extent = canvas_extent(&mut app);
    assert_eq!(
        extent.map(|e| (*e.width(), *e.height())),
        Some((8u8, 8u8)),
        "the boundary extent must track the new grid_size (C4)",
    );
}

/// T4b (C4): switching the THEME re-fills the canvas cells with the NEW theme's default-floor
/// atlas index. Drives the REAL dropdown selection message; asserts the fill index changed to the
/// new theme's default-floor index (value-agnostic — both indices come from the catalogs).
#[test]
fn switching_theme_refills_canvas() {
    let mut app = editor_in_editing();

    let start_theme = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    let target = LevelTheme::Underhive;
    assert_ne!(Some(target), start_theme, "the switch target must differ");

    let target_index = default_floor_index(&app, target);
    assert!(
        target_index.is_some(),
        "the shipped Underhive catalog must declare a default floor (C4)",
    );

    // Emit the real theme dropdown selection message.
    let world = app.world_mut();
    let mut dd = world.query_filtered::<Entity, With<ThemeDropdown>>();
    let control = dd.iter(world).next();
    assert!(control.is_some(), "the theme dropdown must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, target));
    }
    settle(&mut app);

    // The session theme switched and every canvas cell now carries the NEW default-floor index.
    let theme_now = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    assert_eq!(
        theme_now,
        Some(target),
        "the theme switch must update the session"
    );

    let world = app.world_mut();
    let mut q = world.query_filtered::<&ImageNode, With<CanvasCell>>();
    let indices: Vec<Option<usize>> = q
        .iter(world)
        .map(|node| node.texture_atlas.as_ref().map(|atlas| atlas.index))
        .collect();
    assert!(
        !indices.is_empty(),
        "the canvas must have filled cells after the switch"
    );
    if let Some(expected) = target_index {
        for idx in indices {
            assert_eq!(
                idx,
                Some(*expected),
                "switching the theme must re-fill the cells with the NEW default-floor index (C4)",
            );
        }
    }
}

/// T5 (scroll-parenting guard, mirrors GTW-421/422): a [`ScrollListArea`] must be an ANCESTOR of
/// the [`CanvasRoot`] — the grid hangs inside the [`CanvasRegion`]'s clipping, scrolling viewport
/// (NOT the scroll-list grid ROOT FRAME). Pre-fix (parenting onto the frame) this FAILS.
#[test]
fn canvas_lives_inside_the_scroll_area() {
    let mut app = editor_in_editing();

    let world = app.world_mut();
    let mut root_q = world.query_filtered::<Entity, With<CanvasRoot>>();
    let root = root_q.iter(world).next();
    assert!(root.is_some(), "there must be a canvas root to walk from");
    let Some(root) = root else {
        return;
    };

    let world = app.world_mut();
    let mut areas = world.query_filtered::<Entity, With<ScrollListArea>>();
    let area_set: Vec<Entity> = areas.iter(world).collect();
    let mut parents = world.query::<&ChildOf>();

    let mut cursor = Some(root);
    let mut found_in_area = false;
    for _ in 0..64 {
        let Some(current) = cursor else {
            break;
        };
        if area_set.contains(&current) {
            found_in_area = true;
            break;
        }
        cursor = parents.get(world, current).ok().map(ChildOf::parent);
    }
    assert!(
        found_in_area,
        "the canvas root must be parented inside the CanvasRegion's ScrollListArea (the clipping, \
         scrolling viewport) — not under the grid root frame (GTW-421 parenting rule)",
    );
}

/// Find the width + height size-field entities by their [`SizeFieldAxis`] markers.
fn size_fields(app: &mut App) -> (Option<Entity>, Option<Entity>) {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &SizeFieldAxis)>();
    let mut width = None;
    let mut height = None;
    for (entity, axis) in q.iter(world) {
        match axis {
            SizeFieldAxis::Width => width = Some(entity),
            SizeFieldAxis::Height => height = Some(entity),
            SizeFieldAxis::Levels => {}
        }
    }
    (width, height)
}
