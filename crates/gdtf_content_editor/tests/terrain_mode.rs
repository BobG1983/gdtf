//! Headless integration test for the GTW-474 Workbench TERRAIN authoring mode + shell (C4).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the
//! `gdtf_test_utils` config, asset root at the workspace `assets/`), so the editor's actual
//! `Load` pass resolves the shipped registries and its real `Editing` scene spawns the Workbench
//! shell + the TERRAIN form — not a copy.
//!
//! Asserts the C4 contract end-to-end and pin-discriminatingly:
//!
//! - Shell: the top bar + status bar exist and the top bar is OCCLUSION-AWARE — it carries a
//!   higher [`ComputedStackIndex`] than a column node, so it actually draws over the columns
//!   (bevy-traps #8 / GTW-294; the only headless proxy for "the bar renders").
//! - Mode switch: a real [`SegmentSelected`] on the mode tabs flips [`EditorMode`] to TERRAIN and
//!   toggles the per-mode content containers' [`Visibility`] (Terrain visible, Prefab hidden) —
//!   NEVER despawning them.
//! - Footfall gate (C2): the footfall dropdown carries `DisabledButton` for a non-slab kind and
//!   LOSES it when the kind switches to Slab (a real `SegmentSelected` on the kind control).
//! - Kind picker (C2): the kind segmented control offers EXACTLY three segments (Wall / Cover /
//!   Slab).
//! - Save → loader resolves (C3): the live draft projects (the SAME `draft_to_terrain_def` the
//!   save button runs) + serializes, the serialized RON round-trips through the GTW-487 loader's
//!   parser into a `TerrainDefRegistry`, and the registry GAINS the new def.

use bevy::{prelude::*, ui::ComputedStackIndex};
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainDefRegistry, TerrainSimKind};
use gdtf_content_editor::{
    EditorMode, EditorModeTabs, EditorState, EditorStatusBar, EditorTopBar, MapEditorPlugin,
    PrefabModeContent, TerrainDraft, TerrainFootfallPicker, TerrainGraphicChoice,
    TerrainKindChoice, TerrainKindTabs, TerrainModeContent, draft_to_terrain_def,
    serialize_terrain_def,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{DisabledButton, SegmentIndex, SegmentSelected, UiPlugin};

/// A generous frame cap (a SAFETY NET — we poll the `Editing` SIGNAL, not a fixed count).
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer UI harness and advances it into `Editing` with
/// the shell + the TERRAIN form spawned.
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
    // The OnEnter spawns + deferred re-parents + the seed/toggle Updates settle over a few frames.
    for _ in 0..8 {
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

/// The TERRAIN kind-tabs control entity, if spawned.
fn kind_tabs(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<TerrainKindTabs>>()
        .iter(world)
        .next()
}

/// Count entities carrying marker `M`.
fn count<M: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&M>().iter(app.world()).count()
}

/// C1 — the Workbench shell exists and the top bar is OCCLUSION-AWARE: it has a higher paint
/// stack index than a plain column node, so it draws over the columns (bevy-traps #8). The top
/// bar + status bar are present; the prefab mode is the default (its content visible).
#[test]
fn shell_top_bar_and_status_bar_render_above_columns() {
    let mut app = editor_in_editing();

    assert_eq!(
        count::<EditorTopBar>(&mut app),
        1,
        "exactly one top bar must be spawned (C1)",
    );
    assert_eq!(
        count::<EditorStatusBar>(&mut app),
        1,
        "exactly one status bar must be spawned (C1)",
    );
    // The mode tabs (a segmented control with a TERRAIN + a PREFAB segment) live in the top bar.
    assert_eq!(
        count::<EditorModeTabs>(&mut app),
        1,
        "the top-bar mode tabs must be spawned (C1)",
    );

    // OCCLUSION-AWARENESS (bevy-traps #8): the top bar must paint ON A HIGHER stack index than the
    // column content, or it draws nothing despite being Visible + laid out. Paint order in Bevy
    // 0.19 is the `ComputedStackIndex` COMPONENT. Compare the top bar's against a
    // PrefabModeContent container (a default-0-z column).
    let world = app.world_mut();
    let top_bar_z = world
        .query_filtered::<&ComputedStackIndex, With<EditorTopBar>>()
        .iter(world)
        .map(|idx| idx.0)
        .max();
    let column_z = world
        .query_filtered::<&ComputedStackIndex, With<PrefabModeContent>>()
        .iter(world)
        .map(|idx| idx.0)
        .max();
    assert!(
        top_bar_z.is_some(),
        "the top bar must have a ComputedStackIndex (it is in the UI stack)",
    );
    assert!(
        column_z.is_some(),
        "a column content container must have a ComputedStackIndex",
    );
    if let (Some(top), Some(col)) = (top_bar_z, column_z) {
        assert!(
            top > col,
            "the top bar must paint ABOVE the columns (stack index {top} > {col}) so it is not \
             occluded (bevy-traps #8 / GTW-294)",
        );
    }
}

/// C1 — a real mode-tab selection flips `EditorMode` to TERRAIN and toggles the per-mode content
/// containers' Visibility (Terrain visible, Prefab hidden) WITHOUT despawning either (mutate-in-
/// place). The editor opens in PREFAB (default), so the terrain containers start hidden.
#[test]
fn selecting_terrain_tab_toggles_mode_and_visibility() {
    let mut app = editor_in_editing();

    // The editor opens in the default PREFAB mode.
    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Prefab),
        "the editor opens in the default PREFAB mode (C1)",
    );

    let terrain_containers_before = count::<TerrainModeContent>(&mut app);
    let prefab_containers_before = count::<PrefabModeContent>(&mut app);
    assert!(
        terrain_containers_before > 0 && prefab_containers_before > 0,
        "both modes' content containers must be pre-spawned (C1 — toggle Visibility, not spawn)",
    );

    // Drive a real SegmentSelected to the mode tabs' TERRAIN segment (index 0 — the tab order is
    // [TERRAIN, PREFAB]).
    let control = mode_tabs(&mut app);
    assert!(control.is_some(), "the mode tabs must exist");
    let Some(control) = control else { return };
    app.world_mut().write_message(SegmentSelected {
        control,
        index: SegmentIndex::new(EditorMode::Terrain.tab_index()),
    });
    app.update();
    app.update();

    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Terrain),
        "selecting the TERRAIN tab must set EditorMode::Terrain (C1)",
    );

    // The containers are NOT despawned (mutate-in-place — bevy-traps): same counts.
    assert_eq!(
        count::<TerrainModeContent>(&mut app),
        terrain_containers_before,
        "the TERRAIN containers must NOT be despawned on a mode switch (mutate-in-place — C1)",
    );
    assert_eq!(
        count::<PrefabModeContent>(&mut app),
        prefab_containers_before,
        "the PREFAB containers must NOT be despawned on a mode switch (mutate-in-place — C1)",
    );

    // Visibility flipped: TERRAIN containers visible, PREFAB containers hidden.
    let world = app.world_mut();
    let terrain_hidden = world
        .query_filtered::<&Visibility, With<TerrainModeContent>>()
        .iter(world)
        .any(|v| matches!(v, Visibility::Hidden));
    assert!(
        !terrain_hidden,
        "every TERRAIN container must be visible in TERRAIN mode (C1)",
    );
    let world = app.world_mut();
    let prefab_all_hidden = world
        .query_filtered::<&Visibility, With<PrefabModeContent>>()
        .iter(world)
        .all(|v| matches!(v, Visibility::Hidden));
    assert!(
        prefab_all_hidden,
        "every PREFAB container must be hidden in TERRAIN mode (C1)",
    );
}

/// C2 — the kind picker offers EXACTLY three segments (Wall / Cover / Slab — no Floor / Scatter).
/// A structural pin on the closed kind set, asserted on the live segmented control's segment
/// children count.
#[test]
fn kind_picker_offers_exactly_three_kinds() {
    let mut app = editor_in_editing();
    let control = kind_tabs(&mut app);
    assert!(control.is_some(), "the TERRAIN kind tabs must exist");
    let Some(control) = control else { return };

    let world = app.world_mut();
    let segments = world.get::<Children>(control).map_or(0, Children::len);
    assert_eq!(
        segments, 3,
        "the kind picker must offer EXACTLY Wall / Cover / Slab (3 segments — no Floor/Scatter, \
         C2/C6)",
    );
}

/// C2 — the footfall gate: the footfall dropdown is DISABLED for a non-slab kind (a `Wall`
/// default) and ENABLED (loses `DisabledButton`) when the kind switches to Slab — a real
/// `SegmentSelected` on the kind control driving the real `gate_footfall_field` system.
#[test]
fn footfall_gated_to_slab_only() {
    let mut app = editor_in_editing();

    // The default kind is Wall (non-slab), so the footfall dropdown is disabled.
    let disabled_at_wall = app
        .world_mut()
        .query_filtered::<Has<DisabledButton>, With<TerrainFootfallPicker>>()
        .iter(app.world())
        .next();
    assert_eq!(
        disabled_at_wall,
        Some(true),
        "the footfall dropdown must be DISABLED for the default (Wall) kind (C2 — Slab-only)",
    );

    // Switch the kind to Slab (segment index 2 in [Wall, Cover, Slab]).
    let control = kind_tabs(&mut app);
    let Some(control) = control else {
        unreachable!("the kind tabs must exist")
    };
    app.world_mut().write_message(SegmentSelected {
        control,
        index: SegmentIndex::new(TerrainKindChoice::Slab.segment_index()),
    });
    app.update();
    app.update();

    let enabled_at_slab = app
        .world_mut()
        .query_filtered::<Has<DisabledButton>, With<TerrainFootfallPicker>>()
        .iter(app.world())
        .next();
    assert_eq!(
        enabled_at_slab,
        Some(false),
        "the footfall dropdown must be ENABLED (no DisabledButton) once the kind is Slab (C2)",
    );

    // And the draft kind followed the tab selection.
    assert_eq!(
        app.world()
            .get_resource::<TerrainDraft>()
            .map(TerrainDraft::kind),
        Some(TerrainKindChoice::Slab),
        "the Slab tab selection must set the draft kind (C2)",
    );
}

/// C3 — SAVE → the GTW-487 loader resolves: populate the live draft (the same setters the form's
/// commit systems call), project it through the SAME `draft_to_terrain_def` the save button runs,
/// serialize it, round-trip the RON through the loader's parser into a `TerrainDefRegistry`, and
/// assert the registry GAINS the new def with its authored kind. In-memory (no assets pollution).
#[test]
fn saved_terrain_def_resolves_through_the_loader() {
    let mut app = editor_in_editing();

    // Populate the draft as a Slab with a footfall (drives the same setters the commit systems do).
    let key = {
        let world = app.world_mut();
        let Some(mut draft) = world.get_resource_mut::<TerrainDraft>() else {
            unreachable!("the TerrainDraft must exist in Editing")
        };
        draft.set_display_name("Authored Slab".to_owned());
        draft.set_kind(TerrainKindChoice::Slab);
        draft.set_graphic(TerrainGraphicChoice::Slab);
        draft.set_footfall(gdtf_content_editor::FootfallChoice::Metal);
        // Mint the UUID exactly as the save press does (the first-save auto-generate — C2).
        draft.ensure_uuid()
    };
    assert!(
        !key.is_nil(),
        "the first save must MINT a non-nil UUID (C2 — editor-generated)",
    );

    // Project + serialize via the REAL save seam, then RELOAD via the loader's parser.
    let def = {
        let world = app.world();
        let Some(draft) = world.get_resource::<TerrainDraft>() else {
            unreachable!("the TerrainDraft must exist")
        };
        draft_to_terrain_def(draft, key)
    };
    let serialized = serialize_terrain_def(&def);
    assert!(
        serialized.is_ok(),
        "serializing the authored def must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded = ron::de::from_str::<TerrainDef>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the saved def must round-trip through the GTW-487 loader's TerrainDef parser: {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };

    // Build a TerrainDefRegistry the way the loader does (keyed by the def's own UUID) and assert
    // it GAINS the new def (C3 — the loader resolves it).
    let registry = TerrainDefRegistry::new([(reloaded.key, reloaded.clone())]);
    let resolved = registry.def(&key);
    assert!(
        resolved.is_some(),
        "the loader's registry must RESOLVE the saved def by its minted UUID (C3)",
    );
    if let Some(resolved) = resolved {
        assert!(
            matches!(resolved.sim_kind, TerrainSimKind::Slab { .. }),
            "the resolved def must carry the authored Slab kind (C3)",
        );
        assert!(
            matches!(
                &resolved.presenter_kind,
                gdtf_battle_sim::terrain::def::TerrainPresenterKind::Slab {
                    footfall: Some(_),
                    ..
                }
            ),
            "the resolved Slab def must carry the authored footfall (C2/C3 — Slab-only)",
        );
    }
    // Sanity: the projected def equals what the loader parsed (no field dropped).
    assert_eq!(reloaded, def, "the def round-trips byte-for-byte (C3)");
}
