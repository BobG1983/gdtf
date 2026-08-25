use crate::{
    client::{EDITOR_LAST_SAVE, EDITOR_SAVE},
    lifecycle::{ArmorSaveCase, armor_save_case, save_mode, written_path},
    mirror::ModeRow,
    outcome::ran_body,
    rows::{LastSaveOutcomeRow, LastSaveReplyRow, LastSaveRow, SaveOutcomeRow, SaveReplyRow},
    save_fault::SaveFaultRow,
    socket::run_editor,
    support::{TestError, TestResult},
};

fn read_records(
    app: &mut bevy::app::App,
    client: &mut crate::socket::Client,
    arguments: &str,
) -> Result<Vec<LastSaveRow>, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_LAST_SAVE, arguments))?;
    let body: LastSaveReplyRow = ran_body(&reply, EDITOR_LAST_SAVE)?;
    Ok(body.records)
}

fn read_last_save(
    app: &mut bevy::app::App,
    client: &mut crate::socket::Client,
) -> Result<Vec<LastSaveRow>, TestError> {
    read_records(app, client, "()")
}

#[test]
fn last_save_is_empty_until_something_saves_then_carries_that_path() -> TestResult {
    let ArmorSaveCase {
        mut app,
        mut client,
        root: _root,
    } = armor_save_case()?;

    let before = read_last_save(&mut app, &mut client)?;
    assert!(
        before.is_empty(),
        "nothing has saved yet, so the record holds no rows at all: {before:?}",
    );

    let path = written_path(save_mode(&mut app, &mut client, "Armor")?)?;
    let after = read_last_save(&mut app, &mut client)?;

    let Some(row) = after.iter().find(|row| row.mode == ModeRow::Armor) else {
        unreachable!("the Armor save just answered with a path, so it has a row: {after:?}");
    };
    assert_eq!(
        row.outcome,
        LastSaveOutcomeRow::Wrote { path },
        "the record carries the same path the save reply carried — which is what tells two saves \
         under one mode apart",
    );
    Ok(())
}

/// Record a failed Prefab save and a written Armor save, and hand back that Armor path.
fn a_failure_then_a_write() -> Result<(ArmorSaveCase, String), TestError> {
    let mut case = armor_save_case()?;

    let refused = case.client.exchange(
        &mut case.app,
        &run_editor(EDITOR_SAVE, "(mode: Prefab, name: Some(\"\"))"),
    )?;
    let body: SaveReplyRow = ran_body(&refused, EDITOR_SAVE)?;
    assert_eq!(
        body.outcome,
        SaveOutcomeRow::Failed(SaveFaultRow::EmptyName),
        "the empty prefab name is what makes this save fail",
    );

    let path = written_path(save_mode(&mut case.app, &mut case.client, "Armor")?)?;
    Ok((case, path))
}

#[test]
fn a_failed_save_is_readable_through_last_save_too() -> TestResult {
    let (mut case, path) = a_failure_then_a_write()?;
    let records = read_last_save(&mut case.app, &mut case.client)?;

    let Some(failed) = records.iter().find(|row| row.mode == ModeRow::Prefab) else {
        unreachable!("the failed Prefab save is recorded too: {records:?}");
    };
    assert_eq!(
        failed.outcome,
        LastSaveOutcomeRow::Failed(SaveFaultRow::EmptyName),
        "a save failure is typed on the wire, so a client reads why without the log",
    );
    let Some(wrote) = records.iter().find(|row| row.mode == ModeRow::Armor) else {
        unreachable!("the successful Armor save is recorded too: {records:?}");
    };
    assert_eq!(
        wrote.outcome,
        LastSaveOutcomeRow::Wrote { path },
        "a failed save and a successful one are both readable through this one command",
    );
    Ok(())
}

#[test]
fn naming_a_mode_answers_that_mode_and_leaves_every_other_record_out() -> TestResult {
    let (mut case, path) = a_failure_then_a_write()?;
    let records = read_records(&mut case.app, &mut case.client, "(mode: Some(Armor))")?;

    assert!(
        records.iter().any(|row| row.mode == ModeRow::Armor
            && row.outcome == LastSaveOutcomeRow::Wrote { path: path.clone() }),
        "the mode asked for has a record, so the reply carries it: {records:?}",
    );
    assert!(
        !records.iter().any(|row| row.mode == ModeRow::Prefab),
        "the Prefab save is recorded and was not asked for, so naming a mode is a filter rather \
         than a hint the command may ignore: {records:?}",
    );
    Ok(())
}
