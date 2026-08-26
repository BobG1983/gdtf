use gdtf_content_editor::{DEFAULT_EFFECT, EditorMode};

use crate::{
    bad_arguments::bad_arguments_detail,
    outcome::unavailable_code,
    refusal::refusal_note,
    rows::ListRow,
    setup::{form_tab_app_and_client, injury_draft, list_op, try_list_op},
    support::TestResult,
};

#[test]
fn adding_an_effect_seeds_the_forms_own_default() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    let before = injury_draft(&app)?.effects().len();

    let added = list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    assert_eq!(added.list, ListRow::InjuryEffects);
    assert_eq!(
        added.members.len(),
        before + 1,
        "the reply reads the whole list back, one longer than it was",
    );
    let effects = injury_draft(&app)?.effects().to_vec();
    assert_eq!(
        effects.last().copied(),
        Some(DEFAULT_EFFECT),
        "Add appends the form's own default effect, so a handler seeding another template \
         fails here",
    );
    Ok(())
}

#[test]
fn removing_the_last_effect_is_refused_and_leaves_it_in_place() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    let before = injury_draft(&app)?.effects().to_vec();
    assert_eq!(
        before.len(),
        1,
        "a fresh Injury draft holds exactly one effect, which is the minimum this case tests",
    );

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: InjuryEffects, op: Remove(0))",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the form disables its own Remove button while one effect remains, so the write is \
         refused rather than run",
    );
    assert!(
        refusal_note(&reply)?.contains("at least one"),
        "the note names the minimum the list keeps",
    );
    assert_eq!(
        injury_draft(&app)?.effects().to_vec(),
        before,
        "a handler that reaches past the draft and removes the row directly fails here",
    );
    Ok(())
}

#[test]
fn an_effect_is_added_then_removed_by_index() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    assert_eq!(injury_draft(&app)?.effects().len(), 2);

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: InjuryEffects, op: Remove(1))",
    )?;
    assert_eq!(
        removed.members.len(),
        1,
        "the reply reads the shortened list back, got {:?}",
        removed.members,
    );
    assert_eq!(injury_draft(&app)?.effects().len(), 1);
    Ok(())
}

#[test]
fn removing_an_index_past_the_end_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    let before = injury_draft(&app)?.effects().to_vec();
    assert_eq!(
        before.len(),
        2,
        "the list must hold more than the minimum, or the min-one refusal would mask the \
         out-of-bounds answer this case is about",
    );

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: InjuryEffects, op: Remove(7))",
    )?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        injury_draft(&app)?.effects().to_vec(),
        before,
        "the refused op left the effect list exactly as it was",
    );
    Ok(())
}
