//! One weapon delete drops the reference from gang members and replaces it in emplacements.

use gdtf_content_editor::{DeleteOutcome, weapon_save_path_in};

use crate::{
    fixture::{
        FIXTURE_GUN, ORPHAN_GUN, fixture_gang_member, weapon_name, write_fixture_gang,
        write_fixture_weapon,
    },
    harness::{
        OUTCOME_UPDATES, OfferAnswer, WEAPON_FAMILY, editor_on, is_published, member_key,
        run_delete,
    },
    records::{file_text, write_emplacement_def},
};

// The emplacement terrain def mounting the deleted weapon.
const MOUNTING_DEF: &str = "00000000-0000-0000-0000-133000000e02";

#[test]
fn one_weapon_delete_drops_it_from_the_gang_and_replaces_it_in_the_emplacement() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_fixture_weapon(dir.path(), FIXTURE_GUN)
        || !write_fixture_weapon(dir.path(), ORPHAN_GUN)
        || !write_fixture_gang(dir.path(), FIXTURE_GUN)
    {
        return;
    }
    let Some(emplacement) =
        write_emplacement_def(dir.path(), "mounted_def", MOUNTING_DEF, FIXTURE_GUN)
    else {
        return;
    };

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        WEAPON_FAMILY,
        FIXTURE_GUN,
        &OfferAnswer::Confirm(Some(member_key(ORPHAN_GUN))),
    );
    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the weapon delete must settle as Removed within {OUTCOME_UPDATES} updates; published at \
         the end: {}",
        is_published(&app),
    );

    assert!(
        fixture_gang_member(dir.path()).is_some_and(|member| member.weapon.is_none()),
        "the gang member's reference is optional, so the delete drops it rather than replacing \
         it",
    );
    let text = file_text(&emplacement);
    assert!(
        text.contains(ORPHAN_GUN) && !text.contains(FIXTURE_GUN),
        "the emplacement's mounted_weapon is required, so it takes the replacement: {text}",
    );
    assert!(
        !weapon_save_path_in(dir.path(), &weapon_name(FIXTURE_GUN)).exists(),
        "the deleted weapon's own file must be gone",
    );
}

#[test]
fn a_weapon_only_a_gang_member_holds_is_removed_with_no_offer_at_all() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_fixture_weapon(dir.path(), FIXTURE_GUN)
        || !write_fixture_gang(dir.path(), FIXTURE_GUN)
    {
        return;
    }

    let mut app = editor_on(dir.path());
    let settled = run_delete(&mut app, WEAPON_FAMILY, FIXTURE_GUN, &OfferAnswer::Nothing);

    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "every referrer resolves by dropping the reference, so the delete goes through within \
         {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert!(
        !settled.offered,
        "no referring family this weapon has keeps its reference, so no replacement is asked for",
    );
    assert!(
        fixture_gang_member(dir.path()).is_some_and(|member| member.weapon.is_none()),
        "the member that wielded the deleted weapon must be written back holding none",
    );
    assert!(
        !weapon_save_path_in(dir.path(), &weapon_name(FIXTURE_GUN)).exists(),
        "the deleted weapon's own file must be gone",
    );
}
