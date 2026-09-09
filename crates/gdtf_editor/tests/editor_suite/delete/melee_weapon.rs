//! The melee weapon delete writes every holder back with none, and refuses for the default.

use std::path::Path;

use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily};
use gdtf_battle_sim::weapon::{FISTS_KEY, MeleeWeaponRegistry, WeaponName};
use gdtf_editor::{
    DeleteOutcome, DeleteRefusal, DeleteRequest, EditorMcpAssetsRoot, MeleeWeaponDraft,
    draft_to_melee_weapon_spec, melee_weapon_save_path_in, write_melee_weapon_in,
};

use crate::{
    content_shared::{advance::advance_to_published, app::editor_app_with_asset_root},
    delete::{
        fixture::{fixture_gang_member, fixture_gang_path, write_gang_equipped},
        harness::advance_to_outcome,
    },
};

/// The melee weapon the fixture gang's member holds.
const FIXTURE_BLADE: &str = "fixture_blade";

/// The finding family label every melee weapon reference finding carries.
const MELEE_WEAPON_FAMILY: &str = "MeleeWeaponRegistry";

// Write a melee weapon under `root`.
fn write_melee_weapon(root: &Path, stem: &str) -> bool {
    let mut draft = MeleeWeaponDraft::new_melee_weapon();
    draft.set_name(stem.to_owned());
    let (name, spec) = draft_to_melee_weapon_spec(&draft);
    write_melee_weapon_in(root, &name, &spec).is_ok()
}

// The melee key as a registry name.
fn weapon_name(stem: &str) -> WeaponName {
    WeaponName::new(stem.to_owned())
}

// Ask for one melee weapon's delete.
fn request_melee_delete(stem: &str) -> DeleteRequest {
    DeleteRequest::new(
        FindingFamily::new(MELEE_WEAPON_FAMILY.to_owned()),
        ContentMemberKey::new(stem.to_owned()),
    )
}

// Every melee weapon finding the published report holds, whatever it targets.
fn melee_findings(report: &ContentIntegrityReport) -> Vec<String> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match finding {
            ContentFinding::DanglingRef { target, family, .. }
                if **family == *MELEE_WEAPON_FAMILY =>
            {
                Some((**target).clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn deleting_a_melee_weapon_writes_every_holder_back_with_none() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_melee_weapon(dir.path(), FISTS_KEY),
        "the fists record must be planted beside the one under test",
    );
    assert!(
        write_melee_weapon(dir.path(), FIXTURE_BLADE),
        "the fixture melee weapon write must succeed",
    );
    assert!(
        write_gang_equipped(dir.path(), |member| {
            member.melee_weapon = Some(weapon_name(FIXTURE_BLADE));
        }),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(request_melee_delete(FIXTURE_BLADE));
    let outcome = advance_to_outcome(&mut app);
    assert_eq!(
        outcome,
        DeleteOutcome::Removed,
        "the drop rewrites the one holder, so the re-run finds nothing and the record goes",
    );

    assert!(
        fixture_gang_member(dir.path()).is_some_and(|member| member.melee_weapon.is_none()),
        "the gang file on disk must parse to a member holding no melee weapon; found: {:?}",
        fixture_gang_member(dir.path()),
    );
    assert!(
        !melee_weapon_save_path_in(dir.path(), &weapon_name(FIXTURE_BLADE)).exists(),
        "the deleted melee weapon's file must be gone",
    );
    assert_eq!(
        melee_findings(app.world().resource::<ContentIntegrityReport>()),
        Vec::<String>::new(),
        "a member the drop wrote `None` to resolves to `{FISTS_KEY}`, so the re-run must report \
         no melee weapon finding at all, not only none for the deleted key",
    );
}

#[test]
fn deleting_the_default_melee_weapon_is_refused_while_a_member_resolves_to_it() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_melee_weapon(dir.path(), FISTS_KEY),
        "the fists record write must succeed",
    );
    assert!(
        write_gang_equipped(dir.path(), |member| {
            member.melee_weapon = None;
        }),
        "the fixture gang write must succeed",
    );
    let before = std::fs::read(fixture_gang_path(dir.path()));
    assert!(before.is_ok(), "the planted gang file must be readable");

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(request_melee_delete(FISTS_KEY));
    let outcome = advance_to_outcome(&mut app);

    assert!(
        matches!(outcome, DeleteOutcome::Refused(DeleteRefusal::InUse(_))),
        "a member that wrote no melee key still resolves to `{FISTS_KEY}`, and no drop can \
         rewrite a field that already reads `None`; outcome: {outcome:?}",
    );
    assert!(
        app.world()
            .resource::<MeleeWeaponRegistry>()
            .spec(&weapon_name(FISTS_KEY))
            .is_some(),
        "a refused delete must put the record back into MeleeWeaponRegistry",
    );
    assert_eq!(
        std::fs::read(fixture_gang_path(dir.path())).ok(),
        before.ok(),
        "the refused delete must leave the gang file's bytes exactly as they were",
    );
}

#[test]
fn the_default_melee_weapon_is_removed_when_no_gang_resolves_to_it() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_melee_weapon(dir.path(), FISTS_KEY),
        "the fists record write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(request_melee_delete(FISTS_KEY));
    let outcome = advance_to_outcome(&mut app);

    assert_eq!(
        outcome,
        DeleteOutcome::Removed,
        "the refusal falls out of a finding no drop can rewrite, so a root with no gang \
         deletes `{FISTS_KEY}` like any other record",
    );
    assert!(
        !melee_weapon_save_path_in(dir.path(), &weapon_name(FISTS_KEY)).exists(),
        "the removed record's file must be gone",
    );
}

#[test]
fn deleting_a_melee_weapon_is_refused_when_the_root_holds_no_default_to_fall_back_to() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_melee_weapon(dir.path(), FIXTURE_BLADE),
        "the fixture melee weapon write must succeed",
    );
    assert!(
        write_gang_equipped(dir.path(), |member| {
            member.melee_weapon = Some(weapon_name(FIXTURE_BLADE));
        }),
        "the fixture gang write must succeed",
    );
    let before = std::fs::read(fixture_gang_path(dir.path()));
    assert!(before.is_ok(), "the planted gang file must be readable");

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(request_melee_delete(FIXTURE_BLADE));
    let outcome = advance_to_outcome(&mut app);

    assert!(
        matches!(outcome, DeleteOutcome::Refused(DeleteRefusal::InUse(_))),
        "the root holds no `{FISTS_KEY}` record, so writing the member `None` would leave it \
         resolving to a melee key nothing holds; outcome: {outcome:?}",
    );
    assert!(
        app.world()
            .resource::<MeleeWeaponRegistry>()
            .spec(&weapon_name(FIXTURE_BLADE))
            .is_some(),
        "a refused delete must put the record back into MeleeWeaponRegistry",
    );
    assert!(
        melee_weapon_save_path_in(dir.path(), &weapon_name(FIXTURE_BLADE)).exists(),
        "a refused delete must leave the record's own file on disk",
    );
    assert_eq!(
        std::fs::read(fixture_gang_path(dir.path())).ok(),
        before.ok(),
        "the refused delete must leave the gang file's bytes exactly as they were",
    );
}
