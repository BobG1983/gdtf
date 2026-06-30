//! Headless integration test for the GTW-475 Workbench THEME authoring mode (C1/C2/C3/C7c).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the
//! `gdtf_test_utils` config, asset root at the workspace `assets/`), so the editor's actual
//! `Load` pass resolves the shipped registries and its real `Editing` scene spawns the Workbench
//! shell + the THEME form — not a copy. This is the in-engine evidence (verification.md Rule 3)
//! for this view ticket.
//!
//! Asserts, end-to-end and pin-discriminatingly:
//!
//! - 3-way mode switch (C1/C7c): a real [`SegmentSelected`] on the mode tabs' THEME segment flips
//!   [`EditorMode`] to THEME and toggles the per-mode content containers' [`Visibility`] (THEME
//!   visible, TERRAIN + PREFAB hidden) — NEVER despawning any of them.
//! - Form exists (C2): in THEME mode the default-floor picker + the resolved-stats readout exist,
//!   and the terrain-library rows are built from the shipped `TerrainDefRegistry`.
//! - Edit-existing + multi-select + default-floor → resolved-stats (C2/C3/C4/C6): loading the
//!   shipped Underhive theme into the form via the top-bar dropdown populates the multi-select +
//!   the default floor, and the resolved-stats readout resolves a real terrain (the
//!   single-source-of-truth proof — the theme stored a UUID).

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_editor::{
    EditorMode, EditorModeTabs, EditorState, MapEditorPlugin, PrefabModeContent,
    TerrainModeContent, ThemeDefaultFloorPicker, ThemeDraft, ThemeDropdown, ThemeModeContent,
    ThemeResolvedStatsText, ThemeTerrainRow,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{DropdownSelectionChanged, SegmentIndex, SegmentSelected, UiPlugin};

/// A generous frame cap (a SAFETY NET — we poll the `Editing` SIGNAL, not a fixed count).
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer UI harness and advances it into `Editing` with
/// the shell + the THEME form spawned + the registries resolved.
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
        "the editor never reached EditorState::Editing (a genuine load failure, not a \
         frame-budget shortfall)",
    );
    // The OnEnter spawns + deferred re-parents + the seed/toggle/sync Updates settle over a few
    // frames (the library rows are built in Update once the registry resolves).
    for _ in 0..10 {
        app.update();
    }
    app
}

/// The mode-tabs control entity, if exactly one was spawned.
fn mode_tabs(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<EditorModeTabs>>()
        .iter(world)
        .next()
}

/// Count entities carrying marker `M`.
fn count<M: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&M>().iter(app.world()).count()
}

/// Switch the editor into THEME mode via a real `SegmentSelected` on the mode tabs and settle.
fn switch_to_theme(app: &mut App) {
    let Some(control) = mode_tabs(app) else {
        unreachable!("the mode tabs must exist")
    };
    app.world_mut().write_message(SegmentSelected {
        control,
        index: SegmentIndex::new(EditorMode::Theme.tab_index()),
    });
    app.update();
    app.update();
}

/// C1/C7c — a real THEME mode-tab selection flips `EditorMode` to THEME and 3-way-toggles the
/// per-mode content containers' Visibility (THEME visible, TERRAIN + PREFAB hidden) WITHOUT
/// despawning any (mutate-in-place). The editor opens in PREFAB (default), so the theme containers
/// start hidden.
#[test]
fn selecting_theme_tab_toggles_mode_and_visibility() {
    let mut app = editor_in_editing();

    // The editor opens in the default PREFAB mode.
    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Prefab),
        "the editor opens in the default PREFAB mode (C1)",
    );

    let theme_before = count::<ThemeModeContent>(&mut app);
    let terrain_before = count::<TerrainModeContent>(&mut app);
    let prefab_before = count::<PrefabModeContent>(&mut app);
    assert!(
        theme_before > 0 && terrain_before > 0 && prefab_before > 0,
        "all THREE modes' content containers must be pre-spawned (C1 — toggle Visibility, not \
         spawn)",
    );

    switch_to_theme(&mut app);

    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Theme),
        "selecting the THEME tab must set EditorMode::Theme (C1)",
    );

    // No container despawned (mutate-in-place): same counts.
    assert_eq!(
        count::<ThemeModeContent>(&mut app),
        theme_before,
        "THEME containers not despawned"
    );
    assert_eq!(
        count::<TerrainModeContent>(&mut app),
        terrain_before,
        "TERRAIN containers not despawned",
    );
    assert_eq!(
        count::<PrefabModeContent>(&mut app),
        prefab_before,
        "PREFAB containers not despawned",
    );

    // Visibility flipped 3-way: THEME visible, TERRAIN + PREFAB hidden.
    let world = app.world_mut();
    let theme_hidden = world
        .query_filtered::<&Visibility, With<ThemeModeContent>>()
        .iter(world)
        .any(|v| matches!(v, Visibility::Hidden));
    assert!(
        !theme_hidden,
        "every THEME container must be visible in THEME mode (C1/C7c)"
    );
    let world = app.world_mut();
    let terrain_all_hidden = world
        .query_filtered::<&Visibility, With<TerrainModeContent>>()
        .iter(world)
        .all(|v| matches!(v, Visibility::Hidden));
    assert!(
        terrain_all_hidden,
        "every TERRAIN container must be hidden in THEME mode (C7c)"
    );
    let world = app.world_mut();
    let prefab_all_hidden = world
        .query_filtered::<&Visibility, With<PrefabModeContent>>()
        .iter(world)
        .all(|v| matches!(v, Visibility::Hidden));
    assert!(
        prefab_all_hidden,
        "every PREFAB container must be hidden in THEME mode (C7c)"
    );
}

/// C2/C3 — in THEME mode the form's controls exist: exactly one default-floor picker + one
/// resolved-stats readout, and the terrain-library multi-select rows are built from the shipped
/// `TerrainDefRegistry` (one row per loaded terrain def).
#[test]
fn theme_form_controls_and_library_exist() {
    let mut app = editor_in_editing();
    switch_to_theme(&mut app);

    assert_eq!(
        count::<ThemeDefaultFloorPicker>(&mut app),
        1,
        "exactly one default-floor picker must be spawned (C2/C6)",
    );
    assert_eq!(
        count::<ThemeResolvedStatsText>(&mut app),
        1,
        "exactly one resolved-stats readout must be spawned (C3)",
    );

    // The terrain-library rows are built from the loaded TerrainDefRegistry — one per def.
    let registry_len = app
        .world()
        .get_resource::<TerrainDefRegistry>()
        .map_or(0, TerrainDefRegistry::len);
    assert!(
        registry_len > 0,
        "the shipped TerrainDefRegistry must have loaded some defs (a real Load pass)",
    );
    assert_eq!(
        count::<ThemeTerrainRow>(&mut app),
        registry_len,
        "the terrain-library multi-select must offer one row PER loaded terrain def (C2)",
    );
}

/// C2/C3/C4/C6 — load the shipped Underhive theme into the form via the top-bar dropdown: the
/// draft's display name / terrain palette / default floor populate from the registry (C4), and the
/// resolved-stats readout RESOLVES the default-floor terrain against the registry (C3 — the
/// single-source-of-truth proof). Pin-discriminating on the loaded palette + the resolved readout.
#[test]
fn loading_a_theme_populates_the_form_and_resolves_stats() {
    let mut app = editor_in_editing();
    switch_to_theme(&mut app);

    // Resolve the shipped Underhive theme's key + def from the loaded registry.
    let (theme_key, def_terrain, def_floor) = {
        let world = app.world();
        let Some(themes) = world.get_resource::<UuidThemeRegistry>() else {
            unreachable!("the UuidThemeRegistry must have loaded")
        };
        let Some((key, def)) = themes.defs().next() else {
            unreachable!("the shipped registry must have at least one theme")
        };
        (*key, def.terrain.clone(), def.default_floor)
    };
    assert!(
        !def_terrain.is_empty(),
        "the shipped theme must have a non-empty terrain palette (test fixture sanity)",
    );

    // Find the top-bar theme dropdown + drive a real DropdownSelectionChanged<ThemeUuid> to it
    // (the same message the dropdown emits on a click) — the C4 load trigger.
    let dropdown = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<ThemeDropdown>>()
            .iter(world)
            .next()
    };
    let Some(dropdown) = dropdown else {
        unreachable!("the top-bar theme dropdown must exist")
    };
    app.world_mut()
        .write_message(DropdownSelectionChanged::<ThemeUuid>::new(
            dropdown, theme_key,
        ));
    app.update();
    app.update();

    // C4 — the draft populated from the loaded theme.
    let draft_terrain = app
        .world()
        .get_resource::<ThemeDraft>()
        .map(|d| d.terrain().to_vec());
    assert_eq!(
        draft_terrain,
        Some(def_terrain),
        "loading a theme must populate the draft's terrain palette from the registry (C4)",
    );
    let draft_floor = app
        .world()
        .get_resource::<ThemeDraft>()
        .and_then(ThemeDraft::default_floor);
    assert_eq!(
        draft_floor,
        Some(def_floor),
        "loading a theme must populate the draft's default floor from the registry (C4)",
    );

    // C3 — the resolved-stats readout resolved the default-floor terrain (it is no longer the
    // empty placeholder), proving the theme stores a UUID the form RESOLVES.
    let resolved_def = app
        .world()
        .get_resource::<TerrainDefRegistry>()
        .and_then(|t| t.def(&def_floor).map(|d| (*d.display_name).clone()));
    let world = app.world_mut();
    let readout = world
        .query_filtered::<&Text, With<ThemeResolvedStatsText>>()
        .iter(world)
        .map(|t| t.0.clone())
        .next();
    assert!(
        readout.is_some(),
        "the resolved-stats readout text node must exist (C3)"
    );
    if let (Some(readout), Some(name)) = (readout, resolved_def) {
        assert_ne!(
            readout, "Pick a default floor to resolve its stats.",
            "the readout must have RESOLVED the default-floor terrain, not show the placeholder \
             (C3 — single-source-of-truth proof)",
        );
        assert!(
            readout.contains(&name),
            "the resolved readout must name the resolved default-floor def `{name}` (C3): \
             {readout}",
        );
    }
}
