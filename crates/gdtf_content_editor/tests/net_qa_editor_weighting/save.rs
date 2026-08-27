use std::{fs, path::PathBuf};

use gdtf_battle_sim::injuries::InjuryWeighting;
use gdtf_content_editor::EditorQaAssetsRoot;
use tempfile::TempDir;

use crate::{
    mirror::ModeRow,
    rows::{LastSaveOutcomeRow, SaveOutcomeRow},
    save_fault::SaveFaultRow,
    setup::{
        draft_weighting, injury_tab_app_and_client, last_save_rows, save_weighting, select_table,
    },
    support::TestResult,
};

// The one mode `editor.last_save` files a weighting save under.
const ONLY_INJURY: &str = "(mode: Some(Injury))";

#[test]
fn a_qa_save_writes_the_weighting_under_the_qa_assets_root_and_the_file_parses_back() -> TestResult
{
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let root = TempDir::new()?;
    app.world_mut()
        .insert_resource(EditorQaAssetsRoot::new(root.path().to_path_buf()));
    select_table(&mut app, &mut client, "(category: Leg, context: Melee)")?;

    let outcome = save_weighting(&mut app, &mut client)?;

    let SaveOutcomeRow::Wrote { path } = outcome else {
        unreachable!("a save into a writable temp root writes a file, got {outcome:?}");
    };
    let written = PathBuf::from(&path);
    assert!(
        written.starts_with(root.path()),
        "the save must land under the QA assets root the test set, never in the repo's own \
         assets tree: `{path}`",
    );
    let text = fs::read_to_string(&written)?;
    let parsed = ron::de::from_str::<InjuryWeighting>(&text)?;
    assert_eq!(
        parsed,
        draft_weighting(&app)?,
        "the file the save wrote must parse back to the weighting the draft holds",
    );
    Ok(())
}

#[test]
fn a_row_edited_over_the_wire_is_in_the_file_the_save_writes() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let root = TempDir::new()?;
    app.world_mut()
        .insert_resource(EditorQaAssetsRoot::new(root.path().to_path_buf()));
    let loaded = select_table(&mut app, &mut client, "(category: Leg, context: Melee)")?;
    let Some(first) = loaded.minor.first() else {
        unreachable!("the live Leg/Melee table carries Minor rows, got {loaded:?}");
    };
    let raised = first.weight + 1;

    crate::setup::set_field(
        &mut app,
        &mut client,
        &format!("(field: WeightingRowWeight(bucket: Minor, index: 0, weight: {raised}))"),
    )?;
    let outcome = save_weighting(&mut app, &mut client)?;

    let SaveOutcomeRow::Wrote { path } = outcome else {
        unreachable!("a save into a writable temp root writes a file, got {outcome:?}");
    };
    let text = fs::read_to_string(PathBuf::from(&path))?;
    let parsed = ron::de::from_str::<InjuryWeighting>(&text)?;
    let Some(written) = parsed.minor.first() else {
        unreachable!("the saved table keeps the Minor rows it was loaded with: {parsed:?}");
    };
    assert_eq!(
        *written.weight, raised,
        "the save writes the draft the wire edited, so the raised weight must be in the file",
    );
    Ok(())
}

#[test]
fn a_qa_weighting_save_is_recorded_under_the_injury_tab_the_way_the_buttons_save_is() -> TestResult
{
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let root = TempDir::new()?;
    app.world_mut()
        .insert_resource(EditorQaAssetsRoot::new(root.path().to_path_buf()));

    let before = last_save_rows(&mut app, &mut client, ONLY_INJURY)?;
    assert!(
        before.is_empty(),
        "no weighting has been saved yet, so the Injury tab has no record to read: {before:?}",
    );

    let outcome = save_weighting(&mut app, &mut client)?;

    let SaveOutcomeRow::Wrote { path } = outcome else {
        unreachable!("a save into a writable temp root writes a file, got {outcome:?}");
    };
    let after = last_save_rows(&mut app, &mut client, ONLY_INJURY)?;
    let Some(row) = after.iter().find(|row| row.mode == ModeRow::Injury) else {
        unreachable!("the save just answered with a path, so it left an Injury row: {after:?}");
    };
    assert_eq!(
        row.outcome,
        LastSaveOutcomeRow::Wrote { path },
        "the record carries the same path the save reply carried, so a QA-driven weighting save \
         reads back through editor.last_save the way the Save weighting button's does",
    );
    Ok(())
}

#[test]
fn a_writer_that_cannot_make_its_folder_answers_a_failed_outcome_inside_a_ran_reply() -> TestResult
{
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let root = TempDir::new()?;
    // A regular file where the root belongs, so creating the weighting folder under it fails.
    let blocked = root.path().join("assets");
    fs::write(&blocked, "this is a file, not the assets root")?;
    app.world_mut()
        .insert_resource(EditorQaAssetsRoot::new(blocked));

    let outcome = save_weighting(&mut app, &mut client)?;

    let SaveOutcomeRow::Failed(fault) = outcome else {
        unreachable!(
            "no folder can be made under a regular file, so the writer failed: {outcome:?}"
        );
    };
    let SaveFaultRow::Write(detail) = fault else {
        unreachable!("the fault a failed directory create reports is Write, got {fault:?}");
    };
    assert!(
        !detail.is_empty(),
        "the Failed body carries the writer's own error text, so a client can read why nothing \
         was written",
    );
    Ok(())
}

#[test]
fn a_failed_qa_weighting_save_is_recorded_under_the_injury_tab() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let root = TempDir::new()?;
    let blocked = root.path().join("assets");
    fs::write(&blocked, "this is a file, not the assets root")?;
    app.world_mut()
        .insert_resource(EditorQaAssetsRoot::new(blocked));

    let outcome = save_weighting(&mut app, &mut client)?;

    let SaveOutcomeRow::Failed(fault) = outcome else {
        unreachable!(
            "no folder can be made under a regular file, so the writer failed: {outcome:?}"
        );
    };
    let after = last_save_rows(&mut app, &mut client, ONLY_INJURY)?;
    let Some(row) = after.iter().find(|row| row.mode == ModeRow::Injury) else {
        unreachable!("a failed save is still a save the tab records: {after:?}");
    };
    assert_eq!(
        row.outcome,
        LastSaveOutcomeRow::Failed(fault),
        "the record carries the same fault the reply carried, so a QA-driven failure reads back \
         through editor.last_save the way the Save weighting button's does",
    );
    Ok(())
}
