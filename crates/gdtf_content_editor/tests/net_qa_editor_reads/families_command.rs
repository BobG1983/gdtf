use crate::{
    harness::editing_app_and_client,
    load_case::reply_answered_during_load,
    names::EDITOR_FAMILIES,
    outcome::{ran_body, unavailable_code},
    rows::{FamiliesReplyRow, FamilyEntryRow, FamilyRow, FamilyRowEntries},
    socket::run_editor,
    support::TestResult,
};

// Every family the wire enum names, so the count is the enum's own and not a content count.
const EVERY_FAMILY: [FamilyRow; 9] = [
    FamilyRow::Terrain,
    FamilyRow::Theme,
    FamilyRow::Gang,
    FamilyRow::Armor,
    FamilyRow::Injury,
    FamilyRow::Sprite,
    FamilyRow::Attachment,
    FamilyRow::Weapon,
    FamilyRow::MeleeWeapon,
];

fn keys(row: &FamilyRowEntries) -> Vec<String> {
    row.entries
        .iter()
        .map(|entry: &FamilyEntryRow| entry.key.clone())
        .collect()
}

#[test]
fn a_read_without_a_filter_answers_every_family_exactly_once() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let answered: Vec<FamilyRow> = body.families.iter().map(|row| row.family).collect();
    assert_eq!(
        answered,
        EVERY_FAMILY.to_vec(),
        "an unfiltered read answers every family the wire enum names, once each and in its own \
         order, so a client can walk the reply without checking for gaps",
    );
    Ok(())
}

#[test]
fn a_filter_narrows_the_reply_to_that_family_alone() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_FAMILIES, "(family: Some(Armor))"),
    )?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let answered: Vec<FamilyRow> = body.families.iter().map(|row| row.family).collect();
    assert_eq!(
        answered,
        vec![FamilyRow::Armor],
        "a filtered read answers the named family and nothing else. A client asking for one \
         family must not have to sift eight others out",
    );
    Ok(())
}

#[test]
fn every_family_comes_back_sorted_by_key() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let mut compared = 0_usize;
    for row in &body.families {
        let answered = keys(row);
        let mut expected = answered.clone();
        expected.sort();
        assert_eq!(
            answered, expected,
            "{:?} came back out of key order, and every registry is a HashMap underneath, so an \
             unsorted reply differs between two runs of the same editor",
            row.family,
        );
        compared += usize::from(answered.len() > 1);
    }
    assert!(
        compared > 0,
        "no family answered more than one key, so nothing here could tell a sorted reply from \
         an unsorted one. The loaded content is what makes this case discriminate: {body:?}",
    );
    Ok(())
}

#[test]
fn every_entry_carries_a_key_and_a_label() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    for row in &body.families {
        for entry in &row.entries {
            assert!(
                !entry.key.is_empty() && !entry.label.is_empty(),
                "{:?} answered an entry a picker cannot show or a load cannot use: {entry:?}",
                row.family,
            );
        }
    }
    Ok(())
}

#[test]
fn families_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply =
        reply_answered_during_load(run_editor(EDITOR_FAMILIES, "()"), "the editor.families run")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the editor only holds every registry once it enters Editing, so a Load-pass call is \
         refused rather than answering a half-loaded list",
    );
    Ok(())
}
