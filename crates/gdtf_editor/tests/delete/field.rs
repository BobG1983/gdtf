//! The field delete rewrites the situation, the terrain def and the weapon spec that name it.

use std::path::{Path, PathBuf};

use cobalt_ron_assets::write_ron_pretty;
use gdtf_assets::{
    ContentFamily as _, ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily,
};
use gdtf_battle_sim::{
    effects::{
        fields::{FieldDefRegistry, FieldKey},
        on_death::OnDeathEffect,
    },
    metric::{Cell, CellLevel, Level},
    situation::{FieldSpawn, Situation},
    terrain::def::{TerrainDef, TerrainUuid},
    test_support::test_weapon_spec,
    weapon::WeaponSpec,
};
use gdtf_content_families::{TerrainDefsFamily, situation::LoadedSituation};
use gdtf_editor::{
    DeleteOutcome, DeleteRequest, EditorMcpAssetsRoot, FieldDraft, TerrainDraft, draft_to_field,
    draft_to_terrain_def, field_save_path_in, weapon_save_path_in, write_field_in,
    write_situation_in, write_weapon_in,
};

use crate::{
    advance::advance_to_published,
    app::editor_app_with_asset_root,
    fixture::weapon_name,
    harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
};

/// The field every referrer names.
const FIXTURE_FIELD: &str = "fixture_smoke";

/// The file stem the terrain def is planted under, which its display name does not match.
const TERRAIN_STEM: &str = "fixture_brazier";

/// The terrain def's display name, drifted from its file stem on purpose.
const TERRAIN_DISPLAY_NAME: &str = "Brazier Of Another Name";

/// The ranged weapon whose `on_death` leaves the field.
const LEAVING_GUN: &str = "leaving_gun";

/// The finding family label every field reference finding carries.
const FIELD_FAMILY: &str = "FieldDefRegistry";

// The field key as a registry key.
fn field_key() -> FieldKey {
    FieldKey::new(FIXTURE_FIELD.to_owned())
}

// The one on-death list every referrer authors.
fn leaves_the_field() -> Vec<OnDeathEffect> {
    vec![OnDeathEffect::LeaveField { field: field_key() }]
}

// Write the fixture field def under `root`.
fn write_fixture_field(root: &Path) -> bool {
    let mut draft = FieldDraft::new_field();
    draft.set_key(FIXTURE_FIELD.to_owned());
    let (key, def) = draft_to_field(&draft);
    write_field_in(root, &key, &def).is_ok()
}

// The file the terrain def is planted in, named for its stem rather than its display name.
fn terrain_file(root: &Path) -> PathBuf {
    root.join(TerrainDefsFamily::FOLDER)
        .join(format!("{TERRAIN_STEM}.terrain_def.ron"))
}

// Write a terrain def leaving the field behind, under a stem its display name does not match.
fn write_leaving_terrain(root: &Path) -> Option<TerrainUuid> {
    let mut draft = TerrainDraft::default();
    draft.set_display_name(TERRAIN_DISPLAY_NAME.to_owned());
    let uuid = draft.ensure_uuid();
    let mut def = draft_to_terrain_def(&draft, uuid).ok()?;
    def.on_death = leaves_the_field();
    write_ron_pretty(&terrain_file(root), &def).ok()?;
    Some(uuid)
}

// Write a ranged weapon whose on-death leaves the field behind.
fn write_leaving_weapon(root: &Path) -> bool {
    let mut spec = test_weapon_spec();
    spec.on_death = leaves_the_field();
    write_weapon_in(root, &weapon_name(LEAVING_GUN), &spec).is_ok()
}

// Write a situation placing the field on one cell.
fn write_placing_situation(root: &Path) -> bool {
    let situation = Situation {
        fields: vec![FieldSpawn::new(
            CellLevel::new(Cell::new(1, 1), Level::new(0)),
            field_key(),
        )],
        ..Situation::default()
    };
    write_situation_in(root, &situation).is_ok()
}

// A record as the file under `path` holds it.
fn record_in_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let ron = std::fs::read_to_string(path).ok()?;
    ron::from_str(&ron).ok()
}

// Whether an on-death list still leaves the fixture field.
fn leaves_field(effects: &[OnDeathEffect]) -> bool {
    effects.iter().any(
        |effect| matches!(effect, OnDeathEffect::LeaveField { field } if *field == field_key()),
    )
}

#[test]
fn deleting_a_field_rewrites_the_situation_the_terrain_def_and_the_weapon_spec() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_field(dir.path()),
        "the fixture field write must succeed",
    );
    assert!(
        write_leaving_terrain(dir.path()).is_some(),
        "the fixture terrain def write must succeed",
    );
    assert!(
        write_leaving_weapon(dir.path()),
        "the fixture weapon write must succeed",
    );
    assert!(
        write_placing_situation(dir.path()),
        "the fixture situation write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);
    assert!(
        app.world()
            .resource::<LoadedSituation>()
            .fields
            .iter()
            .any(|spawn| spawn.field == field_key()),
        "the planted situation must have loaded with the field it places",
    );

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(FIELD_FAMILY.to_owned()),
        ContentMemberKey::new(FIXTURE_FIELD.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "all three referrers are rewritten, so the delete settles rather than running out of \
         {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    let situation: Option<Situation> =
        record_in_file(&dir.path().join("content/situations/skirmish.ron"));
    assert!(
        situation.is_some_and(|situation| situation.fields.is_empty()),
        "the situation file must come back without the spawn",
    );
    assert!(
        !app.world()
            .resource::<LoadedSituation>()
            .fields
            .iter()
            .any(|spawn| spawn.field == field_key()),
        "the rewrite goes through the resource as well as the file, so LoadedSituation holds \
         the spawn no longer",
    );
    let terrain: Option<TerrainDef> = record_in_file(&terrain_file(dir.path()));
    assert!(
        terrain.is_some_and(|def| !leaves_field(&def.on_death)),
        "the terrain def must be rewritten in the file it was read from; a write derived from \
         its display name lands a second file and leaves this one's LeaveField standing",
    );
    let weapon: Option<WeaponSpec> =
        record_in_file(&weapon_save_path_in(dir.path(), &weapon_name(LEAVING_GUN)));
    assert!(
        weapon.is_some_and(|spec| !leaves_field(&spec.on_death)),
        "the weapon spec's file must come back without the whole LeaveField",
    );
    assert!(
        !field_save_path_in(dir.path(), &field_key()).exists(),
        "the deleted field's file must be gone",
    );
    assert!(
        app.world()
            .resource::<FieldDefRegistry>()
            .def(&field_key())
            .is_none(),
        "the removed record must stay out of FieldDefRegistry",
    );
    let findings = app
        .world()
        .resource::<ContentIntegrityReport>()
        .findings()
        .iter()
        .filter(|finding| {
            matches!(finding, ContentFinding::DanglingRef { family, .. }
                if **family == *FIELD_FAMILY)
        })
        .count();
    assert_eq!(
        findings, 0,
        "every referrer is rewritten in its registry as well as its file, so the re-run \
         reports no field finding",
    );
}
