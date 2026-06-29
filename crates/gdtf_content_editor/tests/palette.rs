//! Headless integration test for the map-editor left tile palette + bottom-right stat region,
//! swept onto the UUID-keyed terrain/theme model (GTW-495 C1).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness, so the
//! editor's actual `Load` pass resolves the shipped UUID-keyed theme + terrain registries and its
//! real `Editing` scene spawns the palette rows, the stat region, and the shared
//! [`MapEditorSession`] — not a copy.
//!
//! Asserts the contract end-to-end and pin-discriminatingly:
//!
//! - T1 (C1): the palette lists every RESOLVABLE terrain of the active theme's palette — one
//!   [`PaletteRow`] per resolvable terrain (count == the resolvable count from the registries),
//!   each carrying its [`TerrainUuid`] and a sprite + name child.
//! - T2 (C1): pressing a row writes that row's [`TerrainUuid`] into
//!   [`MapEditorSession::selected_tile`] AND highlights the row.
//! - T3 (C3): after a selection, the [`StatText`] node shows the selected def's stats.
//! - T4: a [`DropdownSelectionChanged<ThemeUuid>`] for a different theme REPOPULATES the palette.
//! - T5 (scroll-parenting guard): a [`ScrollListArea`] is an ANCESTOR of a palette row.
//!
//! Value-agnostic: it asserts COUNTS / MEMBERSHIP / selection IDENTITY, never an authored
//! magnitude. Panic/expect-free per the workspace lints.

use bevy::{prelude::*, ui::widget::ImageNode};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_content_editor::{
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
    // The OnEnter spawn + the Update `seed_default_theme` + `sync_palette` build the rows + stat
    // text across a few frames (deferred re-parent commands); advance so they are present.
    for _ in 0..6 {
        app.update();
    }
    app
}

/// The number of RESOLVABLE terrain in the active theme's palette — the count the palette must
/// list (a palette entry whose def is missing, or whose graphic role is out of the `TileRoles`
/// vocabulary, still spawns a row but only the def-resolvable count is what `sync_palette` lists).
fn active_theme_resolvable_count(app: &App) -> Option<usize> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let themes = app.world().get_resource::<UuidThemeRegistry>()?;
    let terrain = app.world().get_resource::<TerrainDefRegistry>()?;
    let palette = themes.terrain(&session.theme())?;
    Some(
        palette
            .iter()
            .filter(|key| terrain.def(key).is_some())
            .count(),
    )
}

/// Count the entities carrying the marker `M`.
fn count<M: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&M>().iter(app.world()).count()
}

/// Presses an entity (sets [`Interaction::Pressed`]) and runs ONLY the `Update` schedule so the
/// press-edge driver fires.
fn press(app: &mut App, entity: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
        *interaction = Interaction::Pressed;
    }
    app.world_mut().run_schedule(Update);
}

/// T1 (C1): the palette lists every resolvable terrain of the active theme — one row per
/// resolvable terrain, each row carrying its [`TerrainUuid`] and a sprite + name child.
#[test]
fn palette_lists_every_active_theme_terrain_with_sprite_and_name() {
    let mut app = editor_in_editing();

    let expected = active_theme_resolvable_count(&app);
    assert!(
        expected.is_some_and(|n| n > 0),
        "the active theme's palette must hold at least one resolvable terrain (a real resolve)",
    );

    let row_count = count::<PaletteRow>(&mut app);
    assert_eq!(
        Some(row_count),
        expected,
        "the palette must list one row per resolvable active-theme terrain (C1)",
    );

    // Each row carries a sprite (ImageNode) child and a name (Text) child. The shipped terrain
    // all use TileRoles-vocabulary graphic roles, so the sprite resolves for each row.
    let roles_present = app.world().get_resource::<TileRoles>().is_some();
    assert!(
        roles_present,
        "the editor must have resolved a TileRoles table (C1 wiring)"
    );

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

/// T2 (C1): pressing a palette row writes that row's [`TerrainUuid`] into the session's
/// `selected_tile` AND highlights the row.
#[test]
fn clicking_a_row_sets_selected_tile_and_highlights_it() {
    let mut app = editor_in_editing();

    let world = app.world_mut();
    let mut row_q = world.query::<(Entity, &PaletteRow)>();
    let chosen: Option<(Entity, TerrainUuid)> = row_q
        .iter(world)
        .next()
        .map(|(entity, row)| (entity, row.tile()));
    assert!(chosen.is_some(), "there must be a palette row to click");
    let Some((row_entity, expected_key)) = chosen else {
        return;
    };

    press(&mut app, row_entity);

    // C1a: the session's selected_tile is the clicked row's key.
    let selected = app
        .world()
        .get_resource::<MapEditorSession>()
        .and_then(MapEditorSession::selected_tile);
    assert_eq!(
        selected,
        Some(expected_key),
        "clicking a row must write that row's TerrainUuid into the session (C1)",
    );

    // C1b: exactly the clicked row carries the ActiveButton highlight marker.
    let active_rows: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, (With<PaletteRow>, With<ActiveButton>)>();
        q.iter(world).collect()
    };
    assert_eq!(
        active_rows,
        vec![row_entity],
        "exactly the clicked row must carry the ActiveButton selected-highlight marker (C1)",
    );

    // C1c: after settling, paint_active_buttons paints the selected row the theme's active color.
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
        "the selected row must be painted the theme's active color by paint_active_buttons (C1)",
    );
}

/// T3 (C3): after a selection, the bottom-right stat region's [`StatText`] shows the selected
/// def's stats — it changes from the no-selection placeholder AND names the selected terrain.
#[test]
fn stat_region_shows_selected_tile_stats() {
    let mut app = editor_in_editing();

    let placeholder = stat_text(&mut app);
    assert!(
        placeholder.is_some(),
        "the stat region must hold a StatText node (C3)",
    );

    let world = app.world_mut();
    let mut row_q = world.query::<(Entity, &PaletteRow)>();
    let chosen = row_q
        .iter(world)
        .next()
        .map(|(entity, row)| (entity, row.tile()));
    assert!(chosen.is_some(), "there must be a palette row to select");
    let Some((row_entity, key)) = chosen else {
        return;
    };
    let display_name = app
        .world()
        .get_resource::<TerrainDefRegistry>()
        .and_then(|reg| reg.def(&key).map(|def| (*def.display_name).clone()));
    assert!(
        display_name.is_some(),
        "the selected terrain must resolve in the registry",
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
            "the stat region must name the selected terrain ({name:?}); got {after:?} (C3)",
        );
    }
}

/// The current [`StatText`] string, if a single stat-text node exists.
fn stat_text(app: &mut App) -> Option<String> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Text, With<StatText>>();
    q.iter(world).next().map(|text| text.0.clone())
}

/// T4 (C1): switching the theme (a [`DropdownSelectionChanged<ThemeUuid>`]) REPOPULATES the
/// palette — the new theme's resolvable-terrain count is present.
#[test]
fn switching_theme_repopulates_palette() {
    let mut app = editor_in_editing();

    let start_count = count::<PaletteRow>(&mut app);
    assert!(start_count > 0, "the palette must start populated");

    // Pick a theme DIFFERENT from the session's currently-seeded one.
    let start_theme = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    let target: Option<ThemeUuid> = app
        .world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|r| {
            r.defs()
                .map(|(key, _)| *key)
                .find(|key| Some(*key) != start_theme)
        });
    assert!(
        target.is_some(),
        "the registry must hold a second theme to switch to"
    );
    let Some(target) = target else {
        return;
    };

    // The target theme's resolvable-terrain count.
    let target_count = app
        .world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|themes| {
            app.world()
                .get_resource::<TerrainDefRegistry>()
                .and_then(|terrain| {
                    themes
                        .terrain(&target)
                        .map(|palette| palette.iter().filter(|k| terrain.def(k).is_some()).count())
                })
        });
    assert!(
        target_count.is_some_and(|n| n > 0),
        "the target theme's palette must hold resolvable terrain (the C1 switch target)",
    );

    let world = app.world_mut();
    let mut dd = world.query_filtered::<Entity, With<ThemeDropdown>>();
    let control = dd.iter(world).next();
    assert!(control.is_some(), "the theme dropdown must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, target));
    }
    for _ in 0..4 {
        app.update();
    }

    let theme_now = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    assert_eq!(
        theme_now,
        Some(target),
        "the theme switch must update the session theme (C1)",
    );
    let new_count = count::<PaletteRow>(&mut app);
    assert_eq!(
        Some(new_count),
        target_count,
        "switching the theme must repopulate the palette with the NEW theme's terrain (C1)",
    );
}

/// The `industrial_hive` [`ThemeUuid`] (`terrain/industrial_hive/industrial_hive.terrain_theme.ron`).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// The NS `bulkhead_wall` [`TerrainUuid`] (`terrain/industrial_hive/bulkhead_wall.terrain_def.ron`).
const fn bulkhead_wall_ns() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_0002))
}

/// The GTW-469 EW companion `bulkhead_wall_ew` [`TerrainUuid`]
/// (`terrain/industrial_hive/bulkhead_wall_ew.terrain_def.ron`).
const fn bulkhead_wall_ew() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_0009))
}

/// The set of [`TerrainUuid`]s carried by the current [`PaletteRow`] entities.
fn palette_row_keys(app: &mut App) -> Vec<TerrainUuid> {
    let world = app.world_mut();
    let mut q = world.query::<&PaletteRow>();
    q.iter(world).map(PaletteRow::tile).collect()
}

/// GTW-469 C4 — BOTH the NS and the EW `bulkhead_wall` orientations appear in the editor palette
/// for the `industrial_hive` theme: the editor lists every `TerrainDef` in the theme's palette, so
/// the new EW-wall companion is a placeable, selectable palette entry beside its NS counterpart.
///
/// Value-agnostic: it asserts the PRESENCE of the two known orientation UUIDs as palette rows
/// (membership / identity), never an authored magnitude. Driven through the REAL editor plugin
/// (the same `DropdownSelectionChanged<ThemeUuid>` switch the repopulate test uses) so the rows
/// are spawned by the actual `sync_palette` over the shipped registries — not a copy.
#[test]
fn palette_lists_both_ns_and_ew_wall_orientations() {
    let mut app = editor_in_editing();

    // Switch the active theme to industrial_hive (which authors both bulkhead_wall orientations),
    // independent of whichever theme the session seeded to by display-name sort.
    let ih = industrial_hive_theme();
    let world = app.world_mut();
    let mut dd = world.query_filtered::<Entity, With<ThemeDropdown>>();
    let control = dd.iter(world).next();
    assert!(control.is_some(), "the theme dropdown must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, ih));
    }
    for _ in 0..4 {
        app.update();
    }

    // Precondition: the industrial_hive theme is now active (so the rows are its palette).
    let theme_now = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    assert_eq!(
        theme_now,
        Some(ih),
        "the editor must be on the industrial_hive theme for this orientation-coverage check",
    );

    // Both orientations resolve in the terrain registry (proving the EW def loaded), AND both
    // appear as palette rows (proving the EW def is a placeable, selectable palette entry).
    let resolvable = app.world().get_resource::<TerrainDefRegistry>().map(|reg| {
        reg.def(&bulkhead_wall_ns()).is_some() && reg.def(&bulkhead_wall_ew()).is_some()
    });
    assert_eq!(
        resolvable,
        Some(true),
        "both the NS and EW bulkhead_wall TerrainDefs must resolve in the registry (C4)",
    );

    let keys = palette_row_keys(&mut app);
    assert!(
        keys.contains(&bulkhead_wall_ns()),
        "the NS bulkhead_wall must appear as a palette row (C4)",
    );
    assert!(
        keys.contains(&bulkhead_wall_ew()),
        "the EW bulkhead_wall companion must appear as a palette row beside the NS one (C4)",
    );
}

/// The 6 GTW-470 orientation/direction door + stair [`TerrainUuid`]s authored in the
/// `industrial_hive` theme (`terrain/industrial_hive/*.terrain_def.ron`): `door_ns`, `door_ew`,
/// `stair_ns_up`, `stair_ns_down`, `stair_ew_up`, `stair_ew_down`.
const fn door_stair_uuids() -> [TerrainUuid; 6] {
    [
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_000b)), // door_ns
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_000c)), // door_ew
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_000d)), // stair_ns_up
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_000e)), // stair_ns_down
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_000f)), // stair_ew_up
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_0010)), // stair_ew_down
    ]
}

/// GTW-470 C4 — all 6 orientation/direction door + stair tiles appear in the editor palette for the
/// `industrial_hive` theme: the editor lists every `TerrainDef` in the theme's palette, so each new
/// door/stair tile is a placeable, selectable palette entry.
///
/// Value-agnostic: it asserts the PRESENCE of the 6 known door/stair UUIDs as palette rows
/// (membership / identity), never an authored magnitude. Driven through the REAL editor plugin (the
/// same `DropdownSelectionChanged<ThemeUuid>` switch the other tests use) so the rows are spawned by
/// the actual `sync_palette` over the shipped registries — not a copy.
#[test]
fn palette_lists_door_and_stair_orientation_tiles() {
    let mut app = editor_in_editing();

    // Switch the active theme to industrial_hive (which authors the door/stair tiles).
    let ih = industrial_hive_theme();
    let world = app.world_mut();
    let mut dd = world.query_filtered::<Entity, With<ThemeDropdown>>();
    let control = dd.iter(world).next();
    assert!(control.is_some(), "the theme dropdown must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, ih));
    }
    for _ in 0..4 {
        app.update();
    }

    // Precondition: industrial_hive is active.
    let theme_now = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    assert_eq!(
        theme_now,
        Some(ih),
        "the editor must be on the industrial_hive theme for this door/stair coverage check",
    );

    // Every door/stair tile resolves in the registry (proving the defs loaded) AND appears as a
    // palette row (proving each is a placeable, selectable palette entry).
    for uuid in door_stair_uuids() {
        let resolvable = app
            .world()
            .get_resource::<TerrainDefRegistry>()
            .map(|reg| reg.def(&uuid).is_some());
        assert_eq!(
            resolvable,
            Some(true),
            "the door/stair TerrainDef {uuid:?} must resolve in the registry (C4)",
        );
    }
    let keys = palette_row_keys(&mut app);
    for uuid in door_stair_uuids() {
        assert!(
            keys.contains(&uuid),
            "the door/stair tile {uuid:?} must appear as a palette row (C4)",
        );
    }
}

/// T5 (scroll-parenting guard): a [`ScrollListArea`] must be an ANCESTOR of a palette row.
#[test]
fn palette_rows_live_inside_the_scroll_area_not_the_grid_frame() {
    let mut app = editor_in_editing();

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
