//! Deleting a gang names the replacement in the situation, and refuses a roster that cannot.

use std::path::Path;

use gdtf_battle_sim::ganger::{GangName, GangRegistry};
use gdtf_editor::{
    DeleteOutcome, DeleteRefusal, GangDraft, draft_to_roster, gang_save_path_in, write_gang_in,
};

use crate::delete::{
    harness::{
        GANG_FAMILY, OUTCOME_UPDATES, OfferAnswer, editor_on, is_published, member_key, run_delete,
    },
    records::{file_text, situation_path, write_situation},
};

// The gang the situation names twice and the delete removes.
const DELETED_GANG: &str = "deleted_gang";

// The gang whose roster holds every member the situation asks the deleted gang for.
const MATCHING_GANG: &str = "matching_gang";

// A gang whose roster holds none of those members.
const EMPTY_GANG: &str = "empty_gang";

// The one member the situation places and rosters.
const MEMBER: &str = "Scrap";

// Write a gang at the top of the gangs folder, holding the members named.
fn write_gang(root: &Path, stem: &str, members: &[&str]) -> bool {
    let mut draft = GangDraft::new_gang();
    draft.set_name(stem.to_owned());
    for member in members {
        draft.add_member();
        if let Some(last) = draft.members_mut().last_mut() {
            last.name = gdtf_battle_sim::ganger::GangerName::new((*member).to_owned());
        }
    }
    let (name, roster) = draft_to_roster(&draft);
    write_gang_in(root, &name, &roster).is_ok()
}

// A situation naming the deleted gang in one placement and one roster entry.
fn situation_naming_deleted(root: &Path) -> bool {
    write_situation(
        root,
        &format!(
            "(
    gangers: [(gang: \"{DELETED_GANG}\", member: \"{MEMBER}\", at: (cell: (x: 1, y: 1), level: \
             0), faction: 0, facing: North, stance: Standing, aiming: false, life_state: Alive)],
    rosters: [(gang: \"{DELETED_GANG}\", member: \"{MEMBER}\", faction: 1)],
)
"
        ),
    )
    .is_some()
}

#[test]
fn deleting_a_gang_names_the_replacement_in_every_entry_the_situation_held() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_gang(dir.path(), DELETED_GANG, &[MEMBER])
        || !write_gang(dir.path(), MATCHING_GANG, &[MEMBER])
        || !situation_naming_deleted(dir.path())
    {
        return;
    }

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        GANG_FAMILY,
        DELETED_GANG,
        &OfferAnswer::Confirm(Some(member_key(MATCHING_GANG))),
    );
    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the gang delete must settle as Removed within {OUTCOME_UPDATES} updates; published at \
         the end: {}",
        is_published(&app),
    );

    let text = file_text(&situation_path(dir.path()));
    assert_eq!(
        text.matches(MATCHING_GANG).count(),
        2,
        "the situation names the deleted gang in a placement AND a roster entry, so both must \
         hold the replacement: {text}",
    );
    assert!(
        !text.contains(DELETED_GANG),
        "no entry may keep the deleted gang: {text}",
    );
}

#[test]
fn a_replacement_gang_missing_a_member_is_refused_with_nothing_written() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_gang(dir.path(), DELETED_GANG, &[MEMBER])
        || !write_gang(dir.path(), EMPTY_GANG, &["Someone Else"])
        || !situation_naming_deleted(dir.path())
    {
        return;
    }
    let before = file_text(&situation_path(dir.path()));

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        GANG_FAMILY,
        DELETED_GANG,
        &OfferAnswer::Confirm(Some(member_key(EMPTY_GANG))),
    );

    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Refused(DeleteRefusal::ReplacementLacks(
            member_key(MEMBER)
        ))),
        "a replacement whose roster does not hold every member the situation asks for must be \
         refused, naming the first one missing; published at the end: {}",
        is_published(&app),
    );
    assert_eq!(
        file_text(&situation_path(dir.path())),
        before,
        "the refusal has to come before any referrer is written, so the situation file is \
         byte-unchanged",
    );
    assert!(
        app.world()
            .resource::<GangRegistry>()
            .roster(&GangName::new(DELETED_GANG.to_owned()))
            .is_some(),
        "a refused delete must put the gang back into its registry",
    );
    assert!(
        gang_save_path_in(dir.path(), &GangName::new(DELETED_GANG.to_owned())).exists(),
        "a refused delete must leave the gang's own file on disk",
    );
}
