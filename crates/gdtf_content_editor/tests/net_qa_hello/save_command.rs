use std::path::Path;

use crate::{
    client::EDITOR_SAVE,
    harness::editing_app_and_client,
    lifecycle::{ArmorSaveCase, armor_save_case, save_mode, written_path},
    load_case::reply_answered_during_load,
    outcome::{ran_body, unavailable_code},
    rows::{RefusalRow, SaveOutcomeRow, SaveReplyRow},
    save_fault::SaveFaultRow,
    socket::run_editor,
    support::TestResult,
};

#[test]
fn save_writes_the_armor_draft_under_the_qa_assets_root() -> TestResult {
    let ArmorSaveCase {
        mut app,
        mut client,
        root,
    } = armor_save_case()?;

    let outcome = save_mode(&mut app, &mut client, "Armor")?;

    let path = written_path(outcome)?;
    assert!(
        Path::new(&path).starts_with(root.path()),
        "the save wrote under the QA assets root it was pointed at, not the workspace `assets/`: \
         `{path}` is not under `{}`",
        root.path().display(),
    );
    assert!(
        Path::new(&path).is_file(),
        "the writer left a real file behind at `{path}`",
    );
    Ok(())
}

#[test]
fn a_prefab_save_with_an_empty_name_is_refused_by_the_writers_own_guard() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_SAVE, "(mode: Prefab, name: Some(\"\"))"),
    )?;
    let body: SaveReplyRow = ran_body(&reply, EDITOR_SAVE)?;

    assert_eq!(
        body.outcome,
        SaveOutcomeRow::Failed(SaveFaultRow::EmptyName),
        "an empty prefab name sanitizes to nothing, which the writer's own guard reports before \
         any map conversion runs — a typed outcome inside a successful reply, never Unavailable",
    );
    Ok(())
}

#[test]
fn a_prefab_save_with_no_name_at_all_is_refused_as_the_missing_argument_it_is() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;

    let outcome = save_mode(&mut app, &mut client, "Prefab")?;

    assert_eq!(
        outcome,
        SaveOutcomeRow::Refused(RefusalRow::PrefabNeedsAName),
        "the prefab form carries its own name field, so the name is required here — a client that \
         forgot it is told that, not handed the empty-name failure a client who typed nothing \
         into the box would get",
    );
    Ok(())
}

#[test]
fn save_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        run_editor(EDITOR_SAVE, "(mode: Armor)"),
        "the editor.save run",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "every draft is state-scoped to Editing, so a Load-pass save is refused rather than \
         writing an empty one",
    );
    Ok(())
}

#[test]
fn a_name_on_a_mode_other_than_prefab_is_refused_rather_than_dropped() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_SAVE, "(mode: Armor, name: Some(\"renamed\"))"),
    )?;
    let body: SaveReplyRow = ran_body(&reply, EDITOR_SAVE)?;

    let SaveOutcomeRow::Refused(refusal) = body.outcome else {
        unreachable!("only Prefab carries its own name field, got {body:?}");
    };
    assert_eq!(
        format!("{refusal:?}"),
        "NameBelongsToPrefabOnly",
        "the name is refused, never accepted and dropped, so a client cannot believe it renamed \
         the armor it just saved",
    );
    Ok(())
}
