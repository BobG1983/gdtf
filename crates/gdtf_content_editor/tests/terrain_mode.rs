//! Headless integration test for the GTW-513 (C2) egui TERRAIN form's SAVE path.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (a live
//! [`AssetServer`] rooted at the workspace `assets/`), so the editor's actual `Load` pass resolves
//! the shipped theme + content registries and its real `Editing` scene inserts the
//! [`TerrainDraft`] / [`EditorMode`] / [`MapEditorSession`] model resources — not a copy.
//!
//! Per `verification.md` Rule 3 + the C2.5 contract, this asserts the SAVE + ROUND-TRIP contract:
//! with [`EditorMode::Terrain`] active and a populated [`TerrainDraft`], it drives the SAME save
//! path the egui Save button runs (mint the UUID → [`write_terrain_in`] with a unique
//! [`tempfile::TempDir`] root, which projects via [`draft_to_terrain_def`] + serializes), reads the
//! written `.terrain_def.ron` back off disk, and parses it through the GTW-487 loader's `TerrainDef`
//! deserializer — asserting the reloaded def EQUALS the projected one (every field survives — no
//! pinned magnitudes). Writing into a `TempDir` (NOT into `assets/`) means the test never pollutes
//! the version-controlled workspace tree and cannot race with other tests that glob-scan `assets/`.
//! The `TempDir` auto-cleans on drop. The egui DRAW itself is covered by the gate's Screenshot-QA
//! phase (the egui closure never runs headlessly without a primary egui context). `assert!` +
//! `let … else` keep the test panic-free per the workspace lints (no `unwrap` / `expect` / `panic!`).

#![cfg(debug_assertions)]

use bevy::prelude::*;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{
        TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainSimKind, TerrainTag,
        TerrainUuid,
    },
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_content_editor::{
    EditorMode, EditorState, MapEditorPlugin, MapEditorSession, SaveTerrainError, TerrainDraft,
    TerrainKindChoice, draft_to_terrain_def, serialize_terrain_def, write_terrain_in,
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
/// (`draft_to_terrain_def` → `write_terrain_in`) writes a `.terrain_def.ron` that round-trips
/// through the GTW-487 loader's parser — the written file parses back to a def EQUAL to the
/// projected one. Writes into an isolated `tempfile::TempDir` root so no test ever touches the
/// version-controlled `assets/` tree; the `TempDir` auto-cleans on drop.
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
    draft.set_graphic(TileRole::Cover);
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

    // The projected def — the SAME projection write_terrain_in serializes.
    let Ok(projected) = draft_to_terrain_def(&draft, uuid) else {
        unreachable!("a populated Cover draft must project (no fail-closed gate applies)")
    };

    // Write into an isolated TempDir so the test never touches assets/. The TempDir auto-cleans on
    // drop — no manual cleanup is needed, and a mid-test failure still leaves the workspace clean.
    let Ok(temp_dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    // Drive the real fs write via the root-parameterized core (the SAME logic the egui Save button's
    // `write_terrain` wrapper calls, just pointing at the temp root instead of `assets/`).
    let Ok(path) = write_terrain_in(temp_dir.path(), &draft, uuid, &theme_display) else {
        unreachable!("write_terrain_in must succeed for a named draft into a writable TempDir")
    };

    // Read the written RON back off disk and parse it through the loader's deserializer.
    let Ok(written) = std::fs::read_to_string(&path) else {
        unreachable!("the written terrain def must be readable off disk inside the TempDir")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&written) else {
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

/// GTW-534 C1/C3: after the TERRAIN-tab rework (stat fields + sprite picker moved to the CENTRAL
/// primary region, the `.terrain_def.ron` preview demoted to the LEFT secondary strip), the panel
/// RELOCATION must not break any interaction — a stat-field edit, a sprite-picker selection, and the
/// live-preview re-serialize must all still track the draft.
///
/// Drives the SAME `TerrainDraft` setters the relocated central `primary_panel` controls call (the
/// picker's `set_graphic`, the field stack's `set_kind` / `set_cover_hp` / `set_display_name`), then
/// projects + serializes exactly as the relocated `ron_preview` does each frame — asserting the
/// preview text reflects each edit. Runs on the REAL editor app so the live state-scoped draft is the
/// one the shell would draw, not a copy.
#[test]
fn terrain_tab_rework_keeps_stat_picker_and_preview_wired() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

    // Drive the setters the CENTRAL primary panel's picker + field stack call — a sprite selection
    // (the GTW-516 picker's `set_graphic`) and stat edits (the field stack's `set_kind` / HP / name).
    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("GTW534 Rework Probe".to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_graphic(TileRole::Cover);
    draft.set_cover_hp(gdtf_battle_sim::cover::CoverHp::new(77));

    // Re-read the live draft and project + serialize it EXACTLY as the relocated `ron_preview` does
    // each frame (nil placeholder key until save) — the demoted secondary-strip preview.
    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after the edits")
    };
    let Ok(def) = draft_to_terrain_def(&draft, TerrainUuid::nil()) else {
        unreachable!("a populated Cover draft must project (no fail-closed gate applies)")
    };

    // The sprite-picker selection reached the projected presenter kind (C3: picker still wired).
    let TerrainPresenterKind::Cover { graphic_name } = &def.presenter_kind else {
        unreachable!("a Cover kind must project a Cover presenter kind carrying the picked graphic")
    };
    assert_eq!(
        &**graphic_name,
        TileRole::Cover.as_key(),
        "the sprite-picker selection must reach the projected def's graphic_name (C3)",
    );

    // The stat edit reached the projected sim kind (C3: field stack still wired).
    let TerrainSimKind::Cover { hp, .. } = &def.sim_kind else {
        unreachable!("a Cover kind must project a Cover sim kind carrying the edited HP")
    };
    assert_eq!(
        **hp, 77,
        "the HP stat edit must reach the projected def's cover HP (C3)",
    );

    // The live preview re-serializes the draft and reflects each edit (C3: preview still wired, now
    // in the demoted secondary strip). Assert the serialized RON carries the edited name + graphic.
    let Ok(preview) = serialize_terrain_def(&def) else {
        unreachable!("the preview projection must serialize without error")
    };
    assert!(
        preview.contains("GTW534 Rework Probe"),
        "the demoted RON preview must still re-serialize the edited display name (C3):\n{preview}",
    );
    assert!(
        preview.contains(TileRole::Cover.as_key()),
        "the demoted RON preview must still re-serialize the picked graphic role (C3):\n{preview}",
    );
}

/// C2.1: a fresh `TerrainDraft` projects with the `nil` placeholder key when no UUID is yet minted —
/// the projection the live RON preview shows before the first save (drives `draft_to_terrain_def`
/// directly, no fs). Guards that the preview path never panics on an unminted draft.
#[test]
fn unminted_draft_projects_for_the_preview() {
    let draft = TerrainDraft::default();
    assert_eq!(draft.uuid(), None, "a fresh draft has no minted key");
    let Ok(def) = draft_to_terrain_def(&draft, TerrainUuid::nil()) else {
        unreachable!("a fresh (Wall-kind) draft must project (no fail-closed gate applies)")
    };
    assert_eq!(
        def.key,
        TerrainUuid::nil(),
        "the preview projects with the nil placeholder key until the first save mints one (C2.1)",
    );
}

/// GTW-574 AC3 (live-app half): an Emplacement draft with a mounted weapon selected FROM THE LIVE
/// [`WeaponRegistry`] (the editor's real Load pass resolved it from shipped content) projects
/// through `draft_to_terrain_def` into the Emplacement sim + presenter kinds carrying that EXACT
/// key, writes via `write_terrain_in` into a `TempDir`, reads back, parses through the GTW-487
/// loader's `TerrainDef` schema, and resolves through a [`TerrainDefRegistry`] — the full
/// author-to-registry round-trip for the kind the editor previously COULD NOT author.
#[test]
fn emplacement_save_round_trips_with_mounted_weapon() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

    // Pick the FIRST (sorted) weapon key from the LIVE registry — the same stable order the
    // mounted-weapon dropdown offers. The registry must be non-empty (shipped ranged weapons).
    let weapon = {
        let Some(registry) = app.world().get_resource::<WeaponRegistry>() else {
            unreachable!("the WeaponRegistry must be resolved by Editing (the editor's Load gate)")
        };
        let mut names: Vec<&WeaponName> = registry.keys().collect();
        names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        let Some(first) = names.first() else {
            unreachable!("the shipped weapon registry must offer at least one weapon")
        };
        (*first).clone()
    };

    // Populate the live draft via its real setters (the same writes the egui controls make).
    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("GTW574 Emplacement Probe".to_owned());
    draft.set_kind(TerrainKindChoice::Emplacement);
    draft.set_graphic(TileRole::Emplacement);
    draft.set_mounted_weapon(Some(weapon.clone()));
    let _ = draft.ensure_uuid();

    let theme_display = resolve_theme_display(&app);
    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after population")
    };
    let Some(uuid) = draft.uuid() else {
        unreachable!("ensure_uuid must have minted a key")
    };

    let Ok(temp_dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };
    let Ok(path) = write_terrain_in(temp_dir.path(), &draft, uuid, &theme_display) else {
        unreachable!("write_terrain_in must succeed for an Emplacement draft WITH a weapon")
    };

    // Read back + parse through the loader's schema.
    let Ok(written) = std::fs::read_to_string(&path) else {
        unreachable!("the written terrain def must be readable off disk inside the TempDir")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&written) else {
        unreachable!("the written Emplacement def must parse through the GTW-487 loader schema")
    };
    assert!(
        matches!(
            &reloaded.sim_kind,
            TerrainSimKind::Emplacement { mounted_weapon, .. } if *mounted_weapon == weapon
        ),
        "the reloaded sim kind must be Emplacement carrying the EXACT live-registry weapon key \
         ({weapon:?}) — got {:?}",
        reloaded.sim_kind,
    );
    assert!(
        matches!(
            &reloaded.presenter_kind,
            TerrainPresenterKind::Emplacement { graphic_name }
                if &***graphic_name == TileRole::Emplacement.as_key()
        ),
        "the reloaded presenter kind must be Emplacement carrying the chosen graphic role",
    );

    // And it resolves through a TerrainDefRegistry exactly as the Load pass would (AC3).
    let registry = TerrainDefRegistry::new([(uuid, reloaded)]);
    assert!(
        registry.def(&uuid).is_some(),
        "the round-tripped Emplacement def must resolve through a TerrainDefRegistry (AC3)",
    );
}

/// GTW-574 AC4 (live-app half): FAIL-CLOSED — saving an Emplacement draft with NO selected mounted
/// weapon returns the typed [`SaveTerrainError::MissingMountedWeapon`] and writes NOTHING (the
/// `TempDir` stays empty — no partial file, no silent default).
#[test]
fn emplacement_save_without_weapon_fails_closed_and_writes_nothing() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("GTW574 Unarmed Emplacement".to_owned());
    draft.set_kind(TerrainKindChoice::Emplacement);
    // Deliberately NO mounted weapon selected.
    let _ = draft.ensure_uuid();

    let theme_display = resolve_theme_display(&app);
    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after population")
    };
    let Some(uuid) = draft.uuid() else {
        unreachable!("ensure_uuid must have minted a key")
    };

    let Ok(temp_dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };
    assert_eq!(
        write_terrain_in(temp_dir.path(), &draft, uuid, &theme_display).err(),
        Some(SaveTerrainError::MissingMountedWeapon),
        "saving an Emplacement draft without a mounted weapon must return the typed \
         MissingMountedWeapon error (GTW-574 C6)",
    );
    // Nothing written: the save failed BEFORE any path/dir/file creation, so the TempDir root
    // must still be completely empty.
    let Ok(mut entries) = std::fs::read_dir(temp_dir.path()) else {
        unreachable!("the TempDir root must be readable")
    };
    assert!(
        entries.next().is_none(),
        "a fail-closed Emplacement save must write NOTHING — the TempDir must stay empty (AC4)",
    );
}
