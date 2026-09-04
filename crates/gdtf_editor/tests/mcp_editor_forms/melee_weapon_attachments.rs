//! The Melee Weapon draft's fitted attachments, which the Weapon form draws the same way.

use bevy::app::App;
use gdtf_assets::ContentFolderHandle;
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry};
use gdtf_content_families::AttachmentsFamily;
use gdtf_editor::EditorMode;

use crate::{
    bad_arguments::bad_arguments_detail,
    outcome::unavailable_code,
    rows::ListMemberRow,
    setup::{form_tab_app_and_client, list_op, melee_weapon_draft, try_list_op},
    support::{TestError, TestResult},
};

// The keys the fitted-attachment picker offers, in the order it offers them.
fn picker_keys(app: &App) -> Result<Vec<AttachmentName>, TestError> {
    let Some(registry) = app.world().get_resource::<AttachmentRegistry>() else {
        return Err("the attachment registry is what the fitted rows read".into());
    };
    let mut keys: Vec<AttachmentName> = registry.keys().cloned().collect();
    keys.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(keys)
}

// The first key the fitted-attachment picker offers, which is what Add seeds.
fn a_key(app: &App) -> Result<AttachmentName, TestError> {
    match picker_keys(app)?.first() {
        Some(first) => Ok(first.clone()),
        None => Err("the attachment registry holds no key, so the Add button is disabled".into()),
    }
}

#[test]
fn an_attachment_add_seeds_the_first_key_and_a_remove_takes_it_off() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    let seed = a_key(&app)?;

    let added = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: Add)",
    )?;
    assert_eq!(
        added.members,
        vec![ListMemberRow::Attachment(seed.as_str().to_owned())],
        "Add seeds the first key the picker offers",
    );
    assert_eq!(melee_weapon_draft(&app)?.spec().attachments.len(), 1);

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: Remove(0))",
    )?;
    assert!(removed.members.is_empty());
    assert!(melee_weapon_draft(&app)?.spec().attachments.is_empty(),);
    Ok(())
}

#[test]
fn an_attachment_edit_to_a_key_no_registry_holds_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: Add)",
    )?;
    let before = melee_weapon_draft(&app)?.spec().attachments.to_vec();

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: SetAt(0, Attachment(\"no_such_attachment\")))",
    )?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        melee_weapon_draft(&app)?.spec().attachments.to_vec(),
        before,
        "the refused edit left the fitted list exactly as it was",
    );
    Ok(())
}

#[test]
fn an_attachment_edit_at_an_index_replaces_only_that_key() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    let keys = picker_keys(&app)?;
    let (Some(seed), Some(other)) = (keys.first().cloned(), keys.get(1).cloned()) else {
        return Err("the picker offers fewer than two keys, so no edit can name a second".into());
    };

    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: Add)",
    )?;
    list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: Add)",
    )?;
    assert_eq!(
        melee_weapon_draft(&app)?.spec().attachments.to_vec(),
        vec![seed.clone(), seed.clone()],
        "the case edits the second row, so two seeded rows must be in place first",
    );

    let edited = list_op(
        &mut app,
        &mut client,
        &format!(
            "(list: MeleeWeaponAttachments, op: SetAt(1, Attachment(\"{}\")))",
            other.as_str()
        ),
    )?;
    assert_eq!(
        edited.members,
        vec![
            ListMemberRow::Attachment(seed.as_str().to_owned()),
            ListMemberRow::Attachment(other.as_str().to_owned()),
        ],
        "the reply reads both fitted rows back off the draft, the edited one rewritten",
    );
    assert_eq!(
        melee_weapon_draft(&app)?.spec().attachments.to_vec(),
        vec![seed, other],
        "the edit rewrites index 1 and leaves index 0 as it was, so a handler that appends, or \
         one that writes index 0 whatever the index says, fails here",
    );
    Ok(())
}

#[test]
fn an_attachment_add_with_no_registry_is_missing_model() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    app.world_mut().remove_resource::<AttachmentRegistry>();
    // The content family rebuilds a registry that left the world while its folder handle is held.
    app.world_mut()
        .remove_resource::<ContentFolderHandle<AttachmentsFamily>>();

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: MeleeWeaponAttachments, op: Add)",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "the form disables its own Add button with no registry to read, and a missing registry \
         is a missing model rather than a wrong state",
    );
    assert!(
        melee_weapon_draft(&app)?.spec().attachments.is_empty(),
        "the refused add fitted nothing",
    );
    Ok(())
}
