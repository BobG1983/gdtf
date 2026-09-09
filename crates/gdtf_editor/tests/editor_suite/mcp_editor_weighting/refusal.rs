use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables},
};
use gdtf_editor::EditorMode;

use crate::{
    mcp_editor_weighting::{
        names::{
            EDITOR_SAVE_WEIGHTING, EDITOR_SELECT_WEIGHTING_TABLE, EDITOR_SET_MODE, EDITOR_WEIGHTING,
        },
        setup::{
            draft_weighting, editor_mode, injury_tab_app_and_client, table_of, try_run, weighting,
        },
    },
    mcp_shared::{harness::editing_app_and_client, outcome::unavailable_code, support::TestResult},
};

#[test]
fn a_select_from_the_default_tab_is_refused_and_changes_no_part_of_the_draft() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let before = table_of(&draft_weighting(&app)?);
    assert_eq!(
        editor_mode(&app)?,
        EditorMode::Prefab,
        "a fresh editing session opens on EditorMode's own default, which is not the Injury tab",
    );

    let refused = try_run(
        &mut app,
        &mut client,
        EDITOR_SELECT_WEIGHTING_TABLE,
        "(category: Leg, context: Melee)",
    )?;
    assert_eq!(
        unavailable_code(&refused)?,
        "WrongState",
        "the weighting selectors are drawn only in the Injury arm of the central panel, so the \
         write is refused for the open tab",
    );

    try_run(&mut app, &mut client, EDITOR_SET_MODE, "(mode: Injury)")?;
    let read = weighting(&mut app, &mut client)?;
    assert_eq!(
        read.category.to_category(),
        InjuryCategory::ALL[0],
        "the refused select asked for Leg and wrote nothing, so the draft still holds the \
         category `WeightingDraft::default()` gave it",
    );
    assert_eq!(
        read.context.to_context(),
        DamageContext::ALL[0],
        "the refused select asked for Melee and wrote nothing, so the draft still holds the \
         damage source `WeightingDraft::default()` gave it",
    );
    assert_eq!(
        read, before,
        "the read answers the draft exactly as it stood before the refused select",
    );
    Ok(())
}

#[test]
fn the_two_reads_and_the_save_all_refuse_the_default_tab_too() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;

    for (command, arguments) in [
        (EDITOR_WEIGHTING, "()"),
        (EDITOR_SAVE_WEIGHTING, "()"),
        (
            EDITOR_SELECT_WEIGHTING_TABLE,
            "(category: Head, context: Ranged)",
        ),
    ] {
        let refused = try_run(&mut app, &mut client, command, arguments)?;
        assert_eq!(
            unavailable_code(&refused)?,
            "WrongState",
            "`{command}` belongs to the Injury tab, so it refuses every other one",
        );
    }
    Ok(())
}

#[test]
fn a_select_with_no_injury_tables_in_the_world_reports_a_missing_model() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let before = draft_weighting(&app)?;
    app.world_mut().remove_resource::<InjuryTables>();

    let refused = try_run(
        &mut app,
        &mut client,
        EDITOR_SELECT_WEIGHTING_TABLE,
        "(category: Leg, context: Melee)",
    )?;

    assert_eq!(
        unavailable_code(&refused)?,
        "MissingModel",
        "the tables the selectors read are gone, and the form draws `(loading…)` and touches \
         nothing while that is true",
    );
    assert_eq!(
        draft_weighting(&app)?,
        before,
        "a refused select writes nothing, so the draft still equals the one taken before it",
    );
    Ok(())
}
