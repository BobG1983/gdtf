//! The armor delete writes every wearer back with none, and refuses when it cannot.

use std::path::Path;

use bevy::prelude::ResMut;
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, ContentValidationAppExt as _,
    FindingFamily, FindingReferrer, FindingTarget, ReferenceField, ReferenceKeyScheme,
    ReferringRecord,
};
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};
use gdtf_editor::{
    ArmorDraft, DeleteOutcome, DeleteRefusal, DeleteRequest, EditorMcpAssetsRoot,
    armor_save_path_in, draft_to_spec, write_armor_in,
};

use crate::{
    content_shared::{advance::advance_to_published, app::editor_app_with_asset_root},
    delete::{
        fixture::{FIXTURE_GANG, fixture_gang_member, fixture_gang_path, write_gang_equipped},
        harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
    },
};

/// The armor the fixture gang's member wears.
const FIXTURE_ARMOR: &str = "fixture_plate";

/// The finding family label every armor reference finding carries.
const ARMOR_FAMILY: &str = "ArmorRegistry";

// A referrer no armor drop can rewrite, from a family the gang check never walks.
const EXTRA_REFERRER: &str = "fixture_emplacement";

// Records one more dangling reference to the fixture armor on every Check pass.
fn record_extra_armor_ref(mut report: ResMut<ContentIntegrityReport>) {
    report.record(ContentFinding::DanglingRef {
        referrer:         FindingReferrer::new(format!("terrain def `{EXTRA_REFERRER}` armor")),
        referring_record: ReferringRecord::new(
            FindingFamily::new("TerrainDefRegistry".to_owned()),
            ContentMemberKey::new(EXTRA_REFERRER.to_owned()),
            ReferenceField::new("sim_kind.armor".to_owned()),
        ),
        target:           FindingTarget::new(FIXTURE_ARMOR.to_owned()),
        family:           FindingFamily::new(ARMOR_FAMILY.to_owned()),
        scheme:           ReferenceKeyScheme::FileStem,
    });
}

// Write the fixture armor under `root`.
fn write_fixture_armor(root: &Path) -> bool {
    let mut draft = ArmorDraft::default();
    draft.set_name(FIXTURE_ARMOR.to_owned());
    let (name, spec) = draft_to_spec(&draft);
    write_armor_in(root, &name, &spec).is_ok()
}

// Write a gang whose one member wears the fixture armor.
fn write_gang_wearing_the_armor(root: &Path) -> bool {
    write_gang_equipped(root, |member| {
        member.armor = Some(ArmorName::new(FIXTURE_ARMOR.to_owned()));
    })
}

// The armor key as a registry name.
fn armor_name() -> ArmorName {
    ArmorName::new(FIXTURE_ARMOR.to_owned())
}

// Ask for the fixture armor's delete.
fn request_armor_delete() -> DeleteRequest {
    DeleteRequest::new(
        FindingFamily::new(ARMOR_FAMILY.to_owned()),
        ContentMemberKey::new(FIXTURE_ARMOR.to_owned()),
    )
}

#[test]
fn deleting_armor_writes_every_wearer_back_with_none_and_removes_the_record() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_armor(dir.path()),
        "the fixture armor write must succeed",
    );
    assert!(
        write_gang_wearing_the_armor(dir.path()),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(request_armor_delete());
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "the drop rewrites the one wearer, so the re-run finds nothing and the record goes",
    );

    let member = fixture_gang_member(dir.path());
    assert!(
        member.is_some_and(|member| member.armor.is_none()),
        "the gang file on disk must parse to a member wearing no armor; found: {:?}",
        fixture_gang_member(dir.path()),
    );
    assert!(
        !armor_save_path_in(dir.path(), &armor_name()).exists(),
        "the deleted armor's file must be gone",
    );
    assert!(
        app.world()
            .resource::<ArmorRegistry>()
            .spec(&armor_name())
            .is_none(),
        "the removed record must stay out of ArmorRegistry",
    );
    let findings_for_the_armor = app
        .world()
        .resource::<ContentIntegrityReport>()
        .findings()
        .iter()
        .filter(|finding| match finding {
            ContentFinding::DanglingRef { target, family, .. } => {
                **target == *FIXTURE_ARMOR && **family == *ARMOR_FAMILY
            }
            _ => false,
        })
        .count();
    assert_eq!(
        findings_for_the_armor, 0,
        "the re-run reads the rewritten gang out of GangRegistry with no reload, so no \
         ArmorRegistry finding names the deleted key",
    );
}

#[test]
fn the_in_use_check_names_the_gang_member_the_report_could_not_have_named_before_the_delete() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_armor(dir.path()),
        "the fixture armor write must succeed",
    );
    assert!(
        write_gang_wearing_the_armor(dir.path()),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    app.register_reference_check(record_extra_armor_ref);
    advance_to_published(&mut app);

    app.insert_resource(request_armor_delete());
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    let referring = match outcome {
        Some(DeleteOutcome::Refused(DeleteRefusal::InUse(ref records))) => records.clone(),
        _ => Vec::new(),
    };
    assert!(
        referring.iter().any(|record| *record.key == *FIXTURE_GANG
            && **record.family == *"GangRegistry"
            && *record.field == *"members[].armor"),
        "the in-use check runs with the armor out of its registry, so the wearing gang is a \
         referring record; a check reading only the report as it stood before the delete names \
         no gang at all. Found: {referring:?}",
    );
    assert!(
        referring
            .iter()
            .any(|record| *record.key == *EXTRA_REFERRER),
        "the registered check's record must come back beside the gang's; found: {referring:?}",
    );
    assert!(
        app.world()
            .resource::<ArmorRegistry>()
            .spec(&armor_name())
            .is_some(),
        "a refused delete must put the record back into ArmorRegistry",
    );
    assert!(
        armor_save_path_in(dir.path(), &armor_name()).exists(),
        "a refused delete must leave the record's file on disk",
    );
}

#[test]
fn a_gang_rewrite_that_cannot_be_written_leaves_the_armor_and_its_file_in_place() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_armor(dir.path()),
        "the fixture armor write must succeed",
    );
    assert!(
        write_gang_wearing_the_armor(dir.path()),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    let gang_file = fixture_gang_path(dir.path());
    assert!(
        std::fs::remove_file(&gang_file).is_ok(),
        "the loaded gang's file must be removable before the directory takes its place",
    );
    assert!(
        std::fs::create_dir(&gang_file).is_ok(),
        "a directory at the gang file's path is what makes every rewrite of it fail",
    );

    app.insert_resource(request_armor_delete());
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    assert!(
        matches!(
            outcome,
            Some(DeleteOutcome::Refused(DeleteRefusal::InUse(_)))
        ),
        "the wearing gang is a referring record the rewrite could not resolve, so the delete \
         is refused; outcome: {outcome:?}",
    );
    assert!(
        app.world()
            .resource::<ArmorRegistry>()
            .spec(&armor_name())
            .is_some(),
        "a rewrite that cannot be written leaves the record restored in ArmorRegistry; \
         outcome: {outcome:?}",
    );
    assert!(
        armor_save_path_in(dir.path(), &armor_name()).exists(),
        "the record's own file goes only after every rewrite has been written; outcome: \
         {outcome:?}",
    );
}
