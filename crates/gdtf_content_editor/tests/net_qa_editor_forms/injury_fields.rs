use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{InjuryEffect, StatDelta, StatTarget},
    severity::Severity,
};
use gdtf_content_editor::EditorMode;

use crate::{
    bad_arguments::bad_arguments_detail,
    rows::FieldRow,
    setup::{form_tab_app_and_client, injury_draft, list_op, set_field, try_set_field},
    support::TestResult,
    values::{CategoryRow, InjuryEffectRow, SeverityRow, StatRow},
};

#[test]
fn every_injury_field_writes_the_draft_and_reads_back_as_stored() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;

    let key = set_field(&mut app, &mut client, "(field: InjuryKey(\"cracked_rib\"))")?;
    assert_eq!(key.field, FieldRow::InjuryKey("cracked_rib".to_owned()));
    assert_eq!(injury_draft(&app)?.key(), "cracked_rib");

    let name = set_field(
        &mut app,
        &mut client,
        "(field: InjuryName(\"cracked rib\"))",
    )?;
    assert_eq!(name.field, FieldRow::InjuryName("cracked rib".to_owned()));
    assert_eq!(injury_draft(&app)?.def().name.as_str(), "cracked rib");

    let category = set_field(&mut app, &mut client, "(field: InjuryCategory(Leg))")?;
    assert_eq!(category.field, FieldRow::InjuryCategory(CategoryRow::Leg));
    assert_eq!(injury_draft(&app)?.def().category, InjuryCategory::Leg);

    let severity = set_field(&mut app, &mut client, "(field: InjurySeverity(Critical))")?;
    assert_eq!(
        severity.field,
        FieldRow::InjurySeverity(SeverityRow::Critical)
    );
    assert_eq!(injury_draft(&app)?.def().severity, Severity::Critical);

    let popup = set_field(&mut app, &mut client, "(field: InjuryPopupText(\"crack\"))")?;
    assert_eq!(popup.field, FieldRow::InjuryPopupText("crack".to_owned()));
    assert_eq!(injury_draft(&app)?.def().popup_text.as_str(), "crack");

    let log = set_field(&mut app, &mut client, "(field: InjuryLogText(\"a crack\"))")?;
    assert_eq!(log.field, FieldRow::InjuryLogText("a crack".to_owned()));
    assert_eq!(injury_draft(&app)?.def().log_text.as_str(), "a crack");

    let inspect = set_field(
        &mut app,
        &mut client,
        "(field: InjuryInspectText(\"a rib\"))",
    )?;
    assert_eq!(
        inspect.field,
        FieldRow::InjuryInspectText("a rib".to_owned())
    );
    assert_eq!(injury_draft(&app)?.def().inspect_text.as_str(), "a rib");

    let effect = set_field(
        &mut app,
        &mut client,
        "(field: InjuryEffect(index: 0, effect: Bleeding(amount: 4)))",
    )?;
    assert_eq!(
        effect.field,
        FieldRow::InjuryEffect {
            index:  0,
            effect: InjuryEffectRow::Bleeding { amount: 4 },
        },
    );
    assert_eq!(
        injury_draft(&app)?.effects().first().copied(),
        Some(InjuryEffect::Bleeding {
            amount: gdtf_battle_sim::injuries::BleedAmount::new(4),
        }),
    );
    Ok(())
}

#[test]
fn an_effect_write_at_an_index_replaces_only_that_effect() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    let before = injury_draft(&app)?.effects().to_vec();
    assert_eq!(
        before.len(),
        2,
        "the case needs two effects, or writing one index could not prove the other is untouched",
    );

    let written = set_field(
        &mut app,
        &mut client,
        "(field: InjuryEffect(index: 1, effect: Modify(stat: Aim, amount: -3)))",
    )?;
    assert_eq!(
        written.field,
        FieldRow::InjuryEffect {
            index:  1,
            effect: InjuryEffectRow::Modify {
                stat:   StatRow::Aim,
                amount: -3,
            },
        },
    );

    let after = injury_draft(&app)?.effects().to_vec();
    assert_eq!(
        after.first().copied(),
        before.first().copied(),
        "an arm that rebuilds the whole list rather than one row fails here",
    );
    assert_eq!(
        after.get(1).copied(),
        Some(InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-3),
        }),
    );
    Ok(())
}

#[test]
fn a_severity_the_form_does_not_offer_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    let before = injury_draft(&app)?.def().severity;

    let reply = try_set_field(&mut app, &mut client, "(field: InjurySeverity(Fatal))")?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        injury_draft(&app)?.def().severity,
        before,
        "the injury table offers Minor, Major and Critical only, so Fatal never reaches the draft",
    );
    Ok(())
}

#[test]
fn an_effect_write_past_the_end_is_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    let before = injury_draft(&app)?.effects().to_vec();

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: InjuryEffect(index: 4, effect: DisableHand))",
    )?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        injury_draft(&app)?.effects().to_vec(),
        before,
        "the refused write left the effect list exactly as it was",
    );
    Ok(())
}
