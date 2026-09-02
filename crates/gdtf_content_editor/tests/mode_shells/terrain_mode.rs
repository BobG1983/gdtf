//! Terrain mode: save round-trips, picker wiring, emplacement fail-closed.
#![cfg(debug_assertions)]

use bevy::prelude::*;
use gdtf_battle_sim::{
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainSimKind, TerrainTag,
            TerrainUuid, TerrainView, owed_views, owed_views_for,
        },
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_content_editor::{
    EditorMode, SaveTerrainError, TerrainDraft, TerrainKindChoice, draft_to_terrain_def,
    serialize_terrain_def, write_terrain_in,
};

use crate::support::{advance_to_editing, editor_app, resolve_theme_display};

// The sprite key these cases name on a Cover draft's own first owed view.
const COVER_SPRITE: &str = "cover";

// The same, for an Emplacement draft.
const EMPLACEMENT_SPRITE: &str = "emplacement";

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
    draft.set_view(
        TerrainView::Facing(TerrainFacing::North),
        TerrainGraphicKey::new(COVER_SPRITE.to_owned()),
    );
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
    draft.set_view(
        TerrainView::Facing(TerrainFacing::North),
        TerrainGraphicKey::new(COVER_SPRITE.to_owned()),
    );
    draft.set_cover_hp(gdtf_battle_sim::cover::CoverHp::new(77));

    let Some(draft) = app.world().get_resource::<TerrainDraft>().cloned() else {
        unreachable!("the TerrainDraft must be present after the edits")
    };
    let Ok(def) = draft_to_terrain_def(&draft, TerrainUuid::nil()) else {
        unreachable!("a populated Cover draft must project (no fail-closed gate applies)")
    };

    assert!(
        matches!(&def.presenter_kind, TerrainPresenterKind::Cover),
        "a Cover kind must project a Cover presenter kind",
    );
    assert_eq!(
        def.views
            .sprite(TerrainView::Facing(TerrainFacing::North))
            .map(|sprite| (**sprite).clone()),
        Some(COVER_SPRITE.to_owned()),
        "the view-row selection must reach the projected def's `Facing(North)` row (C3)",
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
        preview.contains(COVER_SPRITE),
        "the demoted RON preview must still re-serialize the picked sprite key (C3):\n{preview}",
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
    draft.set_view(
        TerrainView::Facing(TerrainFacing::North),
        TerrainGraphicKey::new(EMPLACEMENT_SPRITE.to_owned()),
    );
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
        matches!(&reloaded.presenter_kind, TerrainPresenterKind::Emplacement),
        "the reloaded presenter kind must be Emplacement",
    );
    let first_owed = owed_views(&reloaded).first().copied();
    assert_eq!(
        first_owed.and_then(|view| reloaded.views.sprite(view).map(|s| (**s).clone())),
        Some(EMPLACEMENT_SPRITE.to_owned()),
        "the reloaded def's first owed view ({first_owed:?}) must name the key the draft set",
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

// The def a draft writes into a TempDir, read back through the loader's own parser.
fn saved_def(app: &App, draft: &TerrainDraft) -> Option<(TerrainDef, tempfile::TempDir)> {
    let theme_display = resolve_theme_display(app);
    let uuid = draft.uuid()?;
    let temp_dir = tempfile::TempDir::new().ok()?;
    let path = write_terrain_in(temp_dir.path(), draft, uuid, &theme_display).ok()?;
    let written = std::fs::read_to_string(&path).ok()?;
    let def = ron::de::from_str::<TerrainDef>(&written).ok()?;
    Some((def, temp_dir))
}

// A named Wall draft the save path accepts, with a uuid already minted.
fn wall_draft(app: &mut App, name: &str) -> Option<TerrainDraft> {
    let mut draft = app.world_mut().get_resource_mut::<TerrainDraft>()?;
    draft.set_display_name(name.to_owned());
    draft.set_kind(TerrainKindChoice::Wall);
    let _ = draft.ensure_uuid();
    app.world().get_resource::<TerrainDraft>().cloned()
}

// The sprite key this test names for one view, distinct per view.
fn view_key(view: TerrainView) -> TerrainGraphicKey {
    TerrainGraphicKey::new(format!("{view:?}").to_lowercase())
}

#[test]
fn every_view_set_on_the_draft_survives_the_save() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let Some(mut draft) = wall_draft(&mut app, "View Probe") else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    let owed = owed_views_for(draft.kind().piece_kind(), draft.tags());
    for view in owed.iter() {
        draft.set_view(*view, view_key(*view));
    }

    let Some((reloaded, _dir)) = saved_def(&app, &draft) else {
        unreachable!("a named Wall draft must write and parse back out of a TempDir")
    };
    let dropped: Vec<TerrainView> = owed
        .iter()
        .filter(|view| reloaded.views.sprite(**view) != Some(&view_key(**view)))
        .copied()
        .collect();
    assert!(
        dropped.is_empty(),
        "every view the draft named must come back off disk naming the key it was set to; these \
         did not: {dropped:?}",
    );
}

#[test]
fn a_draft_with_one_view_set_saves_a_complete_set_off_that_row() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let Some(mut draft) = wall_draft(&mut app, "Unset Views Probe") else {
        unreachable!("the TerrainDraft must be inserted in Editing")
    };
    assert!(
        draft.views().is_empty(),
        "the case starts from a draft with no view set, or the fill below would prove nothing",
    );
    let owed = owed_views_for(draft.kind().piece_kind(), draft.tags());
    let Some(only) = owed.first().copied() else {
        unreachable!("a Wall draft owes at least one view")
    };
    let picked = view_key(only);
    draft.set_view(only, picked.clone());

    let Some((reloaded, _dir)) = saved_def(&app, &draft) else {
        unreachable!("a named Wall draft must write and parse back out of a TempDir")
    };
    let unfilled: Vec<TerrainView> = owed_views(&reloaded)
        .iter()
        .filter(|view| reloaded.views.sprite(**view) != Some(&picked))
        .copied()
        .collect();
    assert!(
        unfilled.is_empty(),
        "a save fills every owed view from the draft's first filled row, so the next def \
         authored in the form passes the coverage check; these views did not: {unfilled:?}",
    );
}
