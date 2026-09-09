use gdtf_battle_sim::{
    effects::fields::{FieldDef, FieldDuration},
    weapon::DamageType,
};
use gdtf_editor::{EditorMode, draft_to_field};

use crate::{
    mcp_editor_forms::{
        names::EDITOR_DRAFT,
        rows::{FieldFormFieldRow, FieldRow},
        setup::{editor_mode, field_draft, set_field, settled_field_app_and_client, try_set_field},
        values::{DamageTypeRow, DurationRow},
    },
    mcp_shared::{
        bad_arguments::bad_arguments_detail,
        draft_reply::{DraftOutcomeRow, DraftReplyRow},
        mirror::ModeRow,
        outcome::ran_body,
        socket::run_editor,
        support::{TestError, TestResult},
    },
};

#[test]
fn set_mode_opens_the_field_tab_and_every_field_arm_writes_its_own_value() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    assert_eq!(
        editor_mode(&app)?,
        EditorMode::Field,
        "editor.set_mode reaches the Field tab, so a mode wire arm that dropped Field would \
         fail here before any write ran",
    );

    let named = set_field(
        &mut app,
        &mut client,
        "(field: Field(Name(\"toxic_sump\")))",
    )?;
    assert_eq!(
        named.field,
        FieldRow::Field(FieldFormFieldRow::Name("toxic_sump".to_owned())),
        "the reply reads the stem back off the draft",
    );
    assert_eq!(field_draft(&app)?.key(), "toxic_sump");

    let damage = set_field(&mut app, &mut client, "(field: Field(Damage(6)))")?;
    assert_eq!(damage.field, FieldRow::Field(FieldFormFieldRow::Damage(6)));
    assert_eq!(*field_draft(&app)?.damage(), 6);

    let channel = set_field(&mut app, &mut client, "(field: Field(DamageType(Chem)))")?;
    assert_eq!(
        channel.field,
        FieldRow::Field(FieldFormFieldRow::DamageType(DamageTypeRow::Chem)),
    );
    assert_eq!(field_draft(&app)?.damage_type(), DamageType::Chem);

    let duration = set_field(&mut app, &mut client, "(field: Field(Duration(Turns(2))))")?;
    assert_eq!(
        duration.field,
        FieldRow::Field(FieldFormFieldRow::Duration(DurationRow::Turns(2))),
    );
    assert!(
        matches!(field_draft(&app)?.duration(), FieldDuration::Turns(turns) if (*turns).get() == 2),
        "the world's own Field draft holds the finite duration the write named",
    );

    let permanent = set_field(&mut app, &mut client, "(field: Field(Duration(Permanent)))")?;
    assert_eq!(
        permanent.field,
        FieldRow::Field(FieldFormFieldRow::Duration(DurationRow::Permanent)),
    );
    assert_eq!(field_draft(&app)?.duration(), FieldDuration::Permanent);

    assert_eq!(
        field_draft(&app)?.key(),
        "toxic_sump",
        "each arm writes only the value it names, so a stem overwritten by a later arm fails \
         here",
    );
    Ok(())
}

#[test]
fn a_zero_turn_duration_is_refused_and_one_turn_is_written() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    let before = field_draft(&app)?;

    let refused = try_set_field(&mut app, &mut client, "(field: Field(Duration(Turns(0))))")?;
    let detail = bad_arguments_detail(&refused)?;
    assert!(
        detail.contains("Turns(0)"),
        "the detail names the duration that never loads, got `{detail}`",
    );
    assert_eq!(
        field_draft(&app)?,
        before,
        "a zero-turn duration is refused rather than clamped to one, so the draft is untouched",
    );

    let written = set_field(&mut app, &mut client, "(field: Field(Duration(Turns(1))))")?;
    assert_eq!(
        written.field,
        FieldRow::Field(FieldFormFieldRow::Duration(DurationRow::Turns(1))),
        "one turn is the smallest count the loader accepts, and it is written unchanged",
    );
    Ok(())
}

#[test]
fn the_field_tab_projects_what_its_own_conversion_builds() -> TestResult {
    let (mut app, mut client) = settled_field_app_and_client()?;
    set_field(
        &mut app,
        &mut client,
        "(field: Field(Name(\"toxic_sump\")))",
    )?;
    set_field(&mut app, &mut client, "(field: Field(Damage(5)))")?;

    let reply = client.exchange(&mut app, &run_editor(EDITOR_DRAFT, "()"))?;
    let body: DraftReplyRow = ran_body(&reply, EDITOR_DRAFT)?;
    assert_eq!(
        body.mode,
        ModeRow::Field,
        "editor.draft names the tab it read, so a client never has to guess which draft it got",
    );
    let DraftOutcomeRow::Ron(text) = &body.outcome else {
        return Err(TestError::from(format!(
            "expected projected RON for the Field draft, got {body:?}"
        )));
    };
    let projected = ron::de::from_str::<FieldDef>(text)?;
    assert_eq!(
        projected,
        draft_to_field(&field_draft(&app)?).1,
        "the text is the second element of the form's own conversion, which is what the writer \
         hands the RON file, so a projection that answered no draft or serialized the key pair \
         fails here",
    );
    Ok(())
}
