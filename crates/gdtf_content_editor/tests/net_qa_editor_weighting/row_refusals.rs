use gdtf_battle_sim::injuries::InjuryRegistry;
use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};

use crate::{
    bad_arguments::bad_arguments_detail,
    harness::editing_app_and_client,
    names::{EDITOR_LIST_OP, EDITOR_SET_FIELD, EDITOR_SET_MODE},
    outcome::unavailable_code,
    setup::{draft_weighting, injury_tab_app_and_client, list_op, try_run},
    support::{TestError, TestResult},
};

// The note a run answered an Unavailable with.
fn refusal_note(reply: &QaResponse) -> Result<String, TestError> {
    let QaResponse::Outcome(CommandOutcome::Unavailable { note, .. }) = reply else {
        return Err(format!("expected an Unavailable outcome, got {reply:?}").into());
    };
    Ok(note.as_str().to_owned())
}

#[test]
fn the_operations_a_bucket_draws_no_control_for_are_bad_arguments_naming_the_fault() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Minor), op: Add)",
    )?;

    for (operation, wanted) in [
        ("Toggle(WeightingRow((injury: \"x\", weight: 1)))", "toggle"),
        (
            "SetAt(0, WeightingRow((injury: \"x\", weight: 1)))",
            "`Weighting(RowInjury(…))`",
        ),
        (
            "SetAt(0, WeightingRow((injury: \"x\", weight: 1)))",
            "`Weighting(RowWeight(…))`",
        ),
        ("MoveUp(0)", "reorder"),
        ("MoveDown(0)", "reorder"),
    ] {
        let reply = try_run(
            &mut app,
            &mut client,
            EDITOR_LIST_OP,
            &format!("(list: WeightingBucket(Minor), op: {operation})"),
        )?;
        let detail = bad_arguments_detail(&reply)?;
        assert!(
            detail.contains(wanted),
            "the bucket draws no control for `{operation}`, and the refusal must say which: \
             `{detail}`",
        );
    }
    Ok(())
}

#[test]
fn an_index_past_the_end_of_a_bucket_is_bad_arguments_saying_how_many_it_holds() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Minor), op: Add)",
    )?;

    let removed = try_run(
        &mut app,
        &mut client,
        EDITOR_LIST_OP,
        "(list: WeightingBucket(Minor), op: Remove(4))",
    )?;
    let detail = bad_arguments_detail(&removed)?;
    assert!(
        detail.contains("holds 1"),
        "the refusal names how many rows the bucket holds, so a client can fix the index in one \
         round trip: `{detail}`",
    );

    let written = try_run(
        &mut app,
        &mut client,
        EDITOR_SET_FIELD,
        "(field: Weighting(RowWeight(bucket: Minor, index: 4, weight: 2)))",
    )?;
    bad_arguments_detail(&written)?;
    assert_eq!(
        draft_weighting(&app)?.minor.len(),
        1,
        "a refused write leaves the bucket exactly as the Add left it",
    );
    Ok(())
}

#[test]
fn an_injury_key_the_registry_does_not_hold_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Minor), op: Add)",
    )?;
    let before = draft_weighting(&app)?;

    let reply = try_run(
        &mut app,
        &mut client,
        EDITOR_SET_FIELD,
        "(field: Weighting(RowInjury(bucket: Minor, index: 0, injury: \"no_such_injury\")))",
    )?;

    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains("no_such_injury"),
        "the row combo only offers registry keys, and the refusal names the one that is not \
         there: `{detail}`",
    );
    assert_eq!(
        draft_weighting(&app)?,
        before,
        "a refused key leaves the row's own key where it was",
    );
    Ok(())
}

#[test]
fn a_severity_the_table_has_no_bucket_for_is_bad_arguments_before_any_handler_runs() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;

    let reply = try_run(
        &mut app,
        &mut client,
        EDITOR_LIST_OP,
        "(list: WeightingBucket(Fatal), op: Add)",
    )?;

    bad_arguments_detail(&reply)?;
    assert!(
        draft_weighting(&app)?.minor.is_empty(),
        "a bucket name the wire does not spell never reaches a handler, so no bucket grew",
    );
    Ok(())
}

#[test]
fn an_add_against_an_empty_injury_registry_reports_a_missing_model() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    app.world_mut().insert_resource(InjuryRegistry::default());

    let reply = try_run(
        &mut app,
        &mut client,
        EDITOR_LIST_OP,
        "(list: WeightingBucket(Minor), op: Add)",
    )?;

    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "the form disables its own Add button while the registry offers no key, so the write is \
         refused rather than seeded with a key nothing resolves",
    );
    assert!(draft_weighting(&app)?.minor.is_empty());
    Ok(())
}

#[test]
fn a_weighting_write_from_another_form_tab_is_refused_and_the_note_names_that_tab() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    try_run(&mut app, &mut client, EDITOR_SET_MODE, "(mode: Sprite)")?;

    for (command, arguments) in [
        (EDITOR_LIST_OP, "(list: WeightingBucket(Minor), op: Add)"),
        (
            EDITOR_SET_FIELD,
            "(field: Weighting(RowWeight(bucket: Minor, index: 0, weight: 2)))",
        ),
    ] {
        let reply = try_run(&mut app, &mut client, command, arguments)?;
        assert_eq!(
            unavailable_code(&reply)?,
            "WrongState",
            "`{command}` reached a weighting arm while another form was open",
        );
        assert!(
            refusal_note(&reply)?.contains("Sprite"),
            "the note names the tab that is open, so a client can see which form it reached",
        );
    }
    assert!(
        draft_weighting(&app)?.minor.is_empty(),
        "a refused foreign-arm write never touches the weighting draft",
    );
    Ok(())
}
