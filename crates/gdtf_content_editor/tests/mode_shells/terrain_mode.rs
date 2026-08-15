//! Terrain mode: save round-trips, picker wiring, emplacement fail-closed.
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

fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn terrain_save_round_trips_through_the_loader() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("Roundtrip Probe".to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_graphic(TileRole::Cover);
    draft.toggle_tag(TerrainTag::BlocksVision);
    draft.toggle_tag(TerrainTag::Indestructible);
    let _ = draft.ensure_uuid();

    let theme_display = resolve_theme_display(&app);
    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after population")
    };
    let Some(uuid) = draft.uuid() else {
        unreachable!("ensure_uuid must have minted a key")
    };

    let Ok(projected) = draft_to_terrain_def(&draft, uuid) else {
        unreachable!("a populated Cover draft must project (no fail-closed gate applies)")
    };

    let Ok(temp_dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    let Ok(path) = write_terrain_in(temp_dir.path(), &draft, uuid, &theme_display) else {
        unreachable!("write_terrain_in must succeed for a named draft into a writable TempDir")
    };

    let Ok(written) = std::fs::read_to_string(&path) else {
        unreachable!("the written terrain def must be readable off disk inside the TempDir")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&written) else {
        unreachable!(
            "the written terrain def must round-trip through the TerrainDef deserializer (the \
              loader's parser)",
        )
    };
    assert_eq!(
        reloaded, projected,
        "the reloaded TerrainDef must equal the projected one — every field survives the egui Save \
         round-trip (C2.5)",
    );
}

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

#[test]
fn terrain_tab_rework_keeps_stat_picker_and_preview_wired() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("Rework Probe".to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_graphic(TileRole::Cover);
    draft.set_cover_hp(gdtf_battle_sim::cover::CoverHp::new(77));

    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after the edits")
    };
    let Ok(def) = draft_to_terrain_def(&draft, TerrainUuid::nil()) else {
        unreachable!("a populated Cover draft must project (no fail-closed gate applies)")
    };

    let TerrainPresenterKind::Cover { graphic_name } = &def.presenter_kind else {
        unreachable!("a Cover kind must project a Cover presenter kind carrying the picked graphic")
    };
    assert_eq!(
        &**graphic_name,
        TileRole::Cover.as_key(),
        "the sprite-picker selection must reach the projected def's graphic_name (C3)",
    );

    let TerrainSimKind::Cover { hp, .. } = &def.sim_kind else {
        unreachable!("a Cover kind must project a Cover sim kind carrying the edited HP")
    };
    assert_eq!(
        **hp, 77,
        "the HP stat edit must reach the projected def's cover HP (C3)",
    );

    let Ok(preview) = serialize_terrain_def(&def) else {
        unreachable!("the preview projection must serialize without error")
    };
    assert!(
        preview.contains("Rework Probe"),
        "the demoted RON preview must still re-serialize the edited display name (C3):\n{preview}",
    );
    assert!(
        preview.contains(TileRole::Cover.as_key()),
        "the demoted RON preview must still re-serialize the picked graphic role (C3):\n{preview}",
    );
}

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

#[test]
fn emplacement_save_round_trips_with_mounted_weapon() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    if let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() {
        *mode = EditorMode::Terrain;
    }

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

    let Some(mut draft) = app.world_mut().get_resource_mut::<TerrainDraft>() else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    draft.set_display_name("Emplacement Probe".to_owned());
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

    let Ok(written) = std::fs::read_to_string(&path) else {
        unreachable!("the written terrain def must be readable off disk inside the TempDir")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&written) else {
        unreachable!("the written Emplacement def must parse through the loader schema")
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

    let registry = TerrainDefRegistry::new([(uuid, reloaded)]);
    assert!(
        registry.def(&uuid).is_some(),
        "the round-tripped Emplacement def must resolve through a TerrainDefRegistry (AC3)",
    );
}

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
    draft.set_display_name("Unarmed Emplacement".to_owned());
    draft.set_kind(TerrainKindChoice::Emplacement);
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
         MissingMountedWeapon error ",
    );
    let Ok(mut entries) = std::fs::read_dir(temp_dir.path()) else {
        unreachable!("the TempDir root must be readable")
    };
    assert!(
        entries.next().is_none(),
        "a fail-closed Emplacement save must write NOTHING — the TempDir must stay empty (AC4)",
    );
}
