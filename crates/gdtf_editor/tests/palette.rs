//! Headless integration test for the GTW-422 map-editor left tile palette + bottom-right stat
//! region.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the
//! same harness the GTW-417 `editor_shell` + GTW-421 `right_panel` tests use), so the editor's
//! actual `Load` pass resolves the shipped theme + catalog registries and its real `Editing`
//! scene spawns the palette rows, the stat region, and the shared [`MapEditorSession`] — not a
//! copy.
//!
//! Asserts the contract end-to-end and pin-discriminatingly:
//!
//! - T1 (C1): the palette lists EVERY tile of the active theme — one [`PaletteRow`] per catalog
//!   tile (count == the theme catalog's tile count), each carrying its [`TileKey`] and an
//!   [`ImageNode`] sprite + a [`Text`] name child.
//! - T2 (C2): pressing a row writes that row's [`TileKey`] into
//!   [`MapEditorSession::selected_tile`] AND highlights the row (its background changes to the
//!   theme's active color).
//! - T3 (C3): after a selection, the [`StatText`] node's text shows the selected tile's stats
//!   (it changes from the no-selection placeholder and names the tile).
//! - T4 (C4): a [`DropdownSelectionChanged<LevelTheme>`] for a different theme REPOPULATES the
//!   palette — the old rows are gone and the new theme's tile count is present.
//! - T5 (scroll-parenting guard, mirrors GTW-421 T4): a [`ScrollListArea`] is an ANCESTOR of a
//!   palette row (the rows hang in the clipping, scrolling viewport, not the grid root frame).
//!
//! Value-agnostic: it asserts COUNTS / MEMBERSHIP / selection IDENTITY (the selected key
//! matches the clicked row's key; the stat text NAMES the selected tile), never an authored
//! magnitude. Panic/expect-free per the workspace lints (the GTW-417 precedent: `assert!` +
//! `if let Some` guards, never `unwrap`/`expect`/`panic`).

use bevy::{prelude::*, ui::widget::ImageNode};
use gdtf_battle_sim::level::{LevelTheme, ThemeCatalogRegistry, TileKey};
use gdtf_editor::{
    EditorState, LeftPaletteRegion, MapEditorPlugin, MapEditorSession, PaletteRow, StatText,
    ThemeDropdown,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{ActiveButton, DropdownSelectionChanged, ScrollListArea, UiPlugin, theme::GdtfTheme};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness and advances it to
/// [`EditorState::Editing`] with the palette rows + stat region + session present.
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
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         theme + registries (a genuine load failure, not a frame-budget shortfall)",
    );
    // The OnEnter spawn + the Update `sync_palette` build the rows + stat text across a few
    // frames (deferred re-parent commands); advance so they are present before we query.
    for _ in 0..6 {
        app.update();
    }
    app
}

/// The number of tiles the active theme's catalog holds — the count the palette must match.
fn active_theme_tile_count(app: &App) -> Option<usize> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let registry = app.world().get_resource::<ThemeCatalogRegistry>()?;
    registry
        .catalog(session.theme())
        .map(|catalog| catalog.tiles().count())
}

/// Count the entities carrying the marker `M`.
fn count<M: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&M>().iter(app.world()).count()
}

/// Presses an entity (sets [`Interaction::Pressed`]) and runs ONLY the `Update` schedule so the
/// `Changed<Interaction> == Pressed` driver ([`select_palette_tile`]) fires on the press edge.
///
/// Runs `Update` directly rather than `app.update()`: under the `DefaultPlugins` headless
/// harness the primary window is absent, so `bevy_ui`'s `ui_focus_system` (in `PreUpdate`)
/// `set_if_neq`s every node's [`Interaction`] back to `None` (no cursor) — a full `app.update()`
/// would clobber the manual `Pressed` BEFORE the palette driver reads it. The dropdown-widget
/// test's `press` precedent.
fn press(app: &mut App, entity: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
        *interaction = Interaction::Pressed;
    }
    app.world_mut().run_schedule(Update);
}

/// T1 (C1): the palette lists EVERY tile of the active theme — one row per catalog tile, each
/// row carrying its [`TileKey`] and a sprite ([`ImageNode`]) + name ([`Text`]) child.
#[test]
fn palette_lists_every_active_theme_tile_with_sprite_and_name() {
    let mut app = editor_in_editing();

    let expected = active_theme_tile_count(&app);
    assert!(
        expected.is_some_and(|n| n > 0),
        "the active theme's catalog must hold at least one tile (a real folder resolve)",
    );

    let row_count = count::<PaletteRow>(&mut app);
    assert_eq!(
        Some(row_count),
        expected,
        "the palette must list one row per active-theme catalog tile (C1)",
    );

    // Each row carries a sprite (ImageNode) child and a name (Text) child — the whole-tile row.
    let world = app.world_mut();
    let mut rows = world.query_filtered::<&Children, With<PaletteRow>>();
    let rows: Vec<Vec<Entity>> = rows
        .iter(world)
        .map(|children| children.iter().collect())
        .collect();
    assert!(!rows.is_empty(), "there must be at least one palette row");

    for children in rows {
        let has_sprite = children
            .iter()
            .any(|child| world.get::<ImageNode>(*child).is_some());
        let has_name = children
            .iter()
            .any(|child| world.get::<Text>(*child).is_some());
        assert!(
            has_sprite,
            "each palette row must carry a tile SPRITE (ImageNode) child (C1)",
        );
        assert!(
            has_name,
            "each palette row must carry a tile NAME (Text) child (C1)",
        );
    }
}

/// T2 (C2): pressing a palette row writes that row's [`TileKey`] into the session's
/// `selected_tile` AND highlights the row — exactly the clicked row carries the
/// [`ActiveButton`] marker (which `gdtf_ui`'s `paint_active_buttons` paints the theme's active
/// color — the sanctioned selected-button highlight) AND, after settling, its background IS the
/// theme's active color. Pin-discriminating: the stored key must EQUAL the clicked row's key,
/// only the clicked row is `ActiveButton`, and its painted color is the active fill.
#[test]
fn clicking_a_row_sets_selected_tile_and_highlights_it() {
    let mut app = editor_in_editing();

    // Pick a row, remember its TileKey.
    let world = app.world_mut();
    let mut row_q = world.query::<(Entity, &PaletteRow)>();
    let chosen: Option<(Entity, TileKey)> = row_q
        .iter(world)
        .next()
        .map(|(entity, row)| (entity, row.tile().clone()));
    assert!(chosen.is_some(), "there must be a palette row to click");
    let Some((row_entity, expected_key)) = chosen else {
        return;
    };

    // Drive the click: set Interaction::Pressed + run Update directly (the headless harness has
    // no real mouse, and a full app.update() would clobber the press in PreUpdate).
    press(&mut app, row_entity);

    // C2a: the session's selected_tile is the clicked row's key.
    let selected = app
        .world()
        .get_resource::<MapEditorSession>()
        .and_then(|s| s.selected_tile().cloned());
    assert_eq!(
        selected,
        Some(expected_key),
        "clicking a row must write that row's TileKey into the session (C2)",
    );

    // C2b: exactly the clicked row carries the ActiveButton highlight marker.
    let active_rows: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, (With<PaletteRow>, With<ActiveButton>)>();
        q.iter(world).collect()
    };
    assert_eq!(
        active_rows,
        vec![row_entity],
        "exactly the clicked row must carry the ActiveButton selected-highlight marker (C2)",
    );

    // C2c: after settling, gdtf_ui's paint_active_buttons paints the selected row the theme's
    // active color (the visible highlight). Settle so the deferred ActiveButton insert + the
    // paint system have run.
    for _ in 0..3 {
        app.update();
    }
    let active = app
        .world()
        .get_resource::<GdtfTheme>()
        .map(|theme| *theme.button.active);
    let post_bg = app
        .world()
        .get::<BackgroundColor>(row_entity)
        .map(|bg| bg.0);
    assert_eq!(
        post_bg, active,
        "the selected row must be painted the theme's active color by paint_active_buttons (C2)",
    );
}

/// T3 (C3): after a selection, the bottom-right stat region's [`StatText`] shows the selected
/// tile's stats — it changes from the no-selection placeholder AND names the selected tile.
#[test]
fn stat_region_shows_selected_tile_stats() {
    let mut app = editor_in_editing();

    // The placeholder text before any selection.
    let placeholder = stat_text(&mut app);
    assert!(
        placeholder.is_some(),
        "the stat region must hold a StatText node (C3)",
    );

    // Select the first row, remember its display name so we can assert the stats name it.
    let world = app.world_mut();
    let mut row_q = world.query::<(Entity, &PaletteRow)>();
    let chosen = row_q
        .iter(world)
        .next()
        .map(|(entity, row)| (entity, row.tile().clone()));
    assert!(chosen.is_some(), "there must be a palette row to select");
    let Some((row_entity, key)) = chosen else {
        return;
    };
    let display_name = app
        .world()
        .get_resource::<ThemeCatalogRegistry>()
        .and_then(|reg| {
            app.world()
                .get_resource::<MapEditorSession>()
                .and_then(|s| reg.catalog(s.theme()))
        })
        .and_then(|cat| cat.tile(&key).map(|tile| (*tile.display_name).clone()));
    assert!(
        display_name.is_some(),
        "the selected tile must resolve in the active catalog",
    );

    press(&mut app, row_entity);

    let after = stat_text(&mut app);
    assert_ne!(
        after, placeholder,
        "selecting a tile must change the stat region from the placeholder (C3)",
    );
    if let (Some(after), Some(name)) = (after, display_name) {
        assert!(
            after.contains(&name),
            "the stat region must name the selected tile ({name:?}); got {after:?} (C3)",
        );
    }
}

/// The current [`StatText`] string, if a single stat-text node exists.
fn stat_text(app: &mut App) -> Option<String> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Text, With<StatText>>();
    q.iter(world).next().map(|text| text.0.clone())
}

/// T4 (C4): switching the theme (a [`DropdownSelectionChanged<LevelTheme>`] from the theme
/// dropdown) REPOPULATES the palette — the old theme's rows are gone and the new theme's tile
/// count is present. Pin-discriminating: it picks a theme whose catalog tile count is known and
/// asserts the row count tracks it after the switch.
#[test]
fn switching_theme_repopulates_palette() {
    let mut app = editor_in_editing();

    let start_count = count::<PaletteRow>(&mut app);
    assert!(start_count > 0, "the palette must start populated");

    // Pick a NON-default theme that has a shipped catalog.
    let target = LevelTheme::Underhive;
    assert_ne!(target, LevelTheme::default());
    let target_count = app
        .world()
        .get_resource::<ThemeCatalogRegistry>()
        .and_then(|reg| reg.catalog(target))
        .map(|cat| cat.tiles().count());
    assert!(
        target_count.is_some_and(|n| n > 0),
        "the shipped Underhive catalog must hold tiles (the C4 switch target)",
    );

    // Find the theme dropdown control + synthesize the real selection message it would emit;
    // the registered apply_theme_selection writes the session theme, sync_palette rebuilds.
    let world = app.world_mut();
    let mut dd = world.query_filtered::<Entity, With<ThemeDropdown>>();
    let control = dd.iter(world).next();
    assert!(control.is_some(), "the theme dropdown must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, target));
    }
    // A couple of frames: apply_theme_selection (session theme), then sync_palette (rebuild),
    // then the deferred re-parent of the new rows.
    for _ in 0..4 {
        app.update();
    }

    // The session theme switched, and the palette now lists the NEW theme's tiles.
    let theme_now = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    assert_eq!(
        theme_now,
        Some(target),
        "the theme switch must update the session theme",
    );
    let new_count = count::<PaletteRow>(&mut app);
    assert_eq!(
        Some(new_count),
        target_count,
        "switching the theme must repopulate the palette with the NEW theme's tiles (C4)",
    );
}

/// T5 (scroll-parenting guard, mirrors GTW-421 T4): a [`ScrollListArea`] must be an ANCESTOR of
/// a palette row — the rows hang inside the [`LeftPaletteRegion`]'s clipping, scrolling viewport
/// (NOT the scroll-list grid ROOT FRAME the marker rides). Pre-fix (parenting onto the frame)
/// this FAILS: no `ScrollListArea` ancestor. Discriminates the GTW-421 bottom-cramp bug.
#[test]
fn palette_rows_live_inside_the_scroll_area_not_the_grid_frame() {
    let mut app = editor_in_editing();

    // The region exists (sanity) and at least one row.
    assert_eq!(
        count::<LeftPaletteRegion>(&mut app),
        1,
        "exactly one left palette region must exist",
    );
    let world = app.world_mut();
    let mut row_q = world.query_filtered::<Entity, With<PaletteRow>>();
    let row = row_q.iter(world).next();
    assert!(row.is_some(), "there must be a palette row to walk from");
    let Some(row) = row else {
        return;
    };

    // Walk up the ChildOf chain; a ScrollListArea must be an ancestor.
    let world = app.world_mut();
    let mut areas = world.query_filtered::<Entity, With<ScrollListArea>>();
    let area_set: Vec<Entity> = areas.iter(world).collect();
    let mut parents = world.query::<&ChildOf>();

    let mut cursor = Some(row);
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
        "palette rows must be parented inside the LeftPaletteRegion's ScrollListArea (the \
         clipping, scrolling viewport) — not under the grid root frame (GTW-421 bottom-cramp)",
    );
}
