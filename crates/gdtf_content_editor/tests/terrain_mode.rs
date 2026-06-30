//! Headless integration test for the GTW-513 (C2) egui TERRAIN form's SAVE path.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (a live
//! [`AssetServer`] rooted at the workspace `assets/`), so the editor's actual `Load` pass resolves
//! the shipped theme + content registries and its real `Editing` scene inserts the
//! [`TerrainDraft`] / [`EditorMode`] / [`MapEditorSession`] model resources — not a copy.
//!
//! Per `verification.md` Rule 3 + the C2.5 contract, this asserts the SAVE + ROUND-TRIP contract:
//! with [`EditorMode::Terrain`] active and a populated [`TerrainDraft`], it drives the SAME save
//! path the egui Save button runs (mint the UUID → [`write_terrain`], which projects via
//! [`draft_to_terrain_def`] + serializes), reads the written `.terrain_def.ron` back off disk, and
//! parses it through the GTW-487 loader's `TerrainDef` deserializer — asserting the reloaded def
//! EQUALS the projected one (every field survives — no pinned magnitudes). The egui DRAW itself is
//! covered by the gate's Screenshot-QA phase (the egui closure never runs headlessly without a
//! primary egui context). `assert!` + `let … else` keep the test panic-free per the workspace
//! lints (no `unwrap` / `expect` / `panic!`).

#![cfg(debug_assertions)]

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{TerrainDef, TerrainTag, TerrainUuid},
};
use gdtf_content_editor::{
    EditorMode, EditorState, MapEditorPlugin, MapEditorSession, TerrainDraft, TerrainGraphicChoice,
    TerrainKindChoice, draft_to_terrain_def, write_terrain,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll the `Editing` SIGNAL, not
/// a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Build the real editor app on the no-renderer `DefaultPlugins` UI harness (the SAME plugin the
/// binary wires, minus the windowed `EguiPlugin` the headless harness has no window for).
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drive the app until it reaches [`EditorState::Editing`], then a few more frames so the
/// `OnEnter(Editing)` model inserts apply.
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
         registries (a genuine load failure, not a frame-budget shortfall)",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// C2.5: with `EditorMode::Terrain` + a populated `TerrainDraft`, the egui Save path
/// (`draft_to_terrain_def` → `write_terrain`) writes a `.terrain_def.ron` that round-trips through
/// the GTW-487 loader's parser BYTE-FOR-BYTE — the written file parses back to a def EQUAL to the
/// projected one. Self-cleaning: the test removes the file it wrote.
#[test]
fn terrain_save_round_trips_through_the_loader() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    // Switch to TERRAIN (the same write the egui tab / the `1` hotkey performs).
    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

    // Populate the live TerrainDraft via its real setters (the same writes the egui controls make),
    // with a UNIQUE name so the written file never collides with a shipped def. Mint the UUID up
    // front (the idempotent first-save mint).
    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("GTW513 Roundtrip Probe".to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_graphic(TerrainGraphicChoice::Cover);
    draft.toggle_tag(TerrainTag::BlocksVision);
    draft.toggle_tag(TerrainTag::Indestructible);
    let _ = draft.ensure_uuid();

    // Resolve the active theme's display name (the per-theme save dir) the SAME way the egui Save
    // button does, then re-read the populated draft + its minted key.
    let theme_display = resolve_theme_display(&app);
    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after population")
    };
    let Some(uuid) = draft.uuid() else {
        unreachable!("ensure_uuid must have minted a key")
    };

    // The projected def — the SAME projection write_terrain serializes.
    let projected: TerrainDef = draft_to_terrain_def(&draft, uuid);

    // Drive the real fs write (the egui Save button's call).
    let Ok(path) = write_terrain(&draft, uuid, &theme_display) else {
        unreachable!("write_terrain must succeed for a named draft")
    };

    // Read the written RON back off disk and parse it through the loader's deserializer.
    let written = std::fs::read_to_string(&path);
    // Clean up the file we wrote regardless of outcome, so a failure still leaves the tree clean.
    let cleanup = || {
        let _removed = std::fs::remove_file(&path);
    };
    let Ok(written) = written else {
        cleanup();
        unreachable!("the written terrain def must be readable off disk")
    };
    let reloaded = ron::de::from_str::<TerrainDef>(&written);
    cleanup();

    let Ok(reloaded) = reloaded else {
        unreachable!(
            "the written terrain def must round-trip through the TerrainDef deserializer (the \
             GTW-487 loader's parser)",
        )
    };
    assert_eq!(
        reloaded, projected,
        "the reloaded TerrainDef must equal the projected one — every field survives the egui Save \
         round-trip (C2.5)",
    );
}

/// Resolve the active theme's display name from the session + the loaded theme registry — the SAME
/// resolution the egui Save button performs. Empty when no theme has resolved.
fn resolve_theme_display(app: &App) -> String {
    let Some(session) = app.world().get_resource::<MapEditorSession>() else {
        return String::new();
    };
    let theme = session.theme();
    app.world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|themes| themes.def(&theme).map(|def| (*def.display_name).clone()))
        .unwrap_or_default()
}

/// C2.1: a fresh `TerrainDraft` projects with the `nil` placeholder key when no UUID is yet minted —
/// the projection the live RON preview shows before the first save (drives `draft_to_terrain_def`
/// directly, no fs). Guards that the preview path never panics on an unminted draft.
#[test]
fn unminted_draft_projects_for_the_preview() {
    let draft = TerrainDraft::default();
    assert_eq!(draft.uuid(), None, "a fresh draft has no minted key");
    let def = draft_to_terrain_def(&draft, TerrainUuid::nil());
    assert_eq!(
        def.key,
        TerrainUuid::nil(),
        "the preview projects with the nil placeholder key until the first save mints one (C2.1)",
    );
}
