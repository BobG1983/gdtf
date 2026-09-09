//! The attachment delete takes its key out of every weapon fitted with it.

use std::path::Path;

use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily};
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentRegistry, FittedAttachments},
    test_support::{test_melee_weapon_spec, test_weapon_spec},
    weapon::{MeleeWeaponSpec, WeaponSpec},
};
use gdtf_editor::{
    AttachmentDraft, DeleteOutcome, DeleteRequest, EditorMcpAssetsRoot, attachment_save_path_in,
    draft_to_attachment_spec, melee_weapon_save_path_in, weapon_save_path_in, write_attachment_in,
    write_melee_weapon_in, write_weapon_in,
};

use crate::{
    content_shared::{advance::advance_to_published, app::editor_app_with_asset_root},
    delete::{
        fixture::weapon_name,
        harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
    },
};

/// The attachment both weapons are fitted with.
const FIXTURE_SIGHT: &str = "fixture_sight";

/// The ranged weapon fitted with the fixture attachment.
const FITTED_GUN: &str = "fitted_gun";

/// The melee weapon fitted with the fixture attachment.
const FITTED_BLADE: &str = "fitted_blade";

/// The finding family label every attachment reference finding carries.
const ATTACHMENT_FAMILY: &str = "AttachmentRegistry";

// The attachment key as a registry name.
fn attachment_name() -> AttachmentName {
    AttachmentName::new(FIXTURE_SIGHT.to_owned())
}

// Write the fixture attachment under `root`.
fn write_fixture_attachment(root: &Path) -> bool {
    let mut draft = AttachmentDraft::new_attachment();
    draft.set_name(FIXTURE_SIGHT.to_owned());
    let (name, spec) = draft_to_attachment_spec(&draft);
    write_attachment_in(root, &name, &spec).is_ok()
}

// Write a ranged and a melee weapon under `root`, both fitted with the attachment.
fn write_fitted_weapons(root: &Path) -> bool {
    let fitted = FittedAttachments::new(vec![attachment_name()]);
    let mut ranged = test_weapon_spec();
    ranged.attachments = fitted.clone();
    let mut melee = test_melee_weapon_spec();
    melee.attachments = fitted;
    write_weapon_in(root, &weapon_name(FITTED_GUN), &ranged).is_ok()
        && write_melee_weapon_in(root, &weapon_name(FITTED_BLADE), &melee).is_ok()
}

// The attachments list the file on disk holds.
fn fitted_in_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let ron = std::fs::read_to_string(path).ok()?;
    ron::from_str(&ron).ok()
}

#[test]
fn deleting_an_attachment_takes_its_key_out_of_the_ranged_and_the_melee_spec() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_attachment(dir.path()),
        "the fixture attachment write must succeed",
    );
    assert!(
        write_fitted_weapons(dir.path()),
        "both fitted weapon writes must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(ATTACHMENT_FAMILY.to_owned()),
        ContentMemberKey::new(FIXTURE_SIGHT.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "both fitted specs are rewritten, so the re-run finds nothing and the record goes",
    );

    let ranged: Option<WeaponSpec> =
        fitted_in_file(&weapon_save_path_in(dir.path(), &weapon_name(FITTED_GUN)));
    assert!(
        ranged.is_some_and(|spec| !spec.attachments.contains(&attachment_name())),
        "the ranged spec's file must come back without the deleted key",
    );
    let melee: Option<MeleeWeaponSpec> = fitted_in_file(&melee_weapon_save_path_in(
        dir.path(),
        &weapon_name(FITTED_BLADE),
    ));
    assert!(
        melee.is_some_and(|spec| !spec.attachments.contains(&attachment_name())),
        "the melee spec's file must come back without the deleted key; rewriting only the \
         ranged spec leaves the melee finding unresolved",
    );
    assert!(
        !attachment_save_path_in(dir.path(), &attachment_name()).exists(),
        "the deleted attachment's file must be gone",
    );
    assert!(
        app.world()
            .resource::<AttachmentRegistry>()
            .spec(&attachment_name())
            .is_none(),
        "the removed record must stay out of AttachmentRegistry",
    );
    let findings = app
        .world()
        .resource::<ContentIntegrityReport>()
        .findings()
        .iter()
        .filter(|finding| {
            matches!(finding, ContentFinding::DanglingRef { family, .. }
                if **family == *ATTACHMENT_FAMILY)
        })
        .count();
    assert_eq!(
        findings, 0,
        "the re-run reads both rewritten specs out of their registries, so no attachment \
         finding stands",
    );
}
