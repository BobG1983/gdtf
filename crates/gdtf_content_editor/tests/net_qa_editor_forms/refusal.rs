use gdtf_content_editor::EditorMode;
use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};

use crate::{
    bad_arguments::bad_arguments_detail,
    outcome::unavailable_code,
    setup::{
        armor_draft, attachment_draft, form_tab_app_and_client, set_field, sprite_draft,
        try_list_op, try_set_field,
    },
    support::{TestError, TestResult},
};

/// The refusal note a run answered with, or why the reply was not a refusal.
pub(crate) fn refusal_note(reply: &QaResponse) -> Result<String, TestError> {
    let QaResponse::Outcome(CommandOutcome::Unavailable { note, .. }) = reply else {
        return Err(format!("expected an Unavailable outcome, got {reply:?}").into());
    };
    Ok(note.as_str().to_owned())
}

#[test]
fn an_attachment_field_on_the_sprite_tab_is_refused_and_the_note_names_the_active_mode()
-> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    let before = sprite_draft(&app)?;

    let reply = try_set_field(&mut app, &mut client, "(field: Attachment(Slot(Rail)))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "a field arm belonging to another form is refused rather than written",
    );
    assert!(
        refusal_note(&reply)?.contains("Sprite"),
        "the note names the tab that is open, so a client can see which form it reached",
    );
    assert_eq!(
        sprite_draft(&app)?,
        before,
        "the refused write left the Sprite draft exactly as it was",
    );
    Ok(())
}

#[test]
fn a_frame_op_while_animation_is_off_is_refused_and_the_note_names_the_gate() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    assert!(
        !sprite_draft(&app)?.is_animated(),
        "a fresh Sprite draft is not animated, which is the closed gate this case needs",
    );

    let reply = try_list_op(&mut app, &mut client, "(list: SpriteFrames, op: Add)")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the frame controls are drawn only while animation is on, so the write is refused \
         rather than run through a setter that would silently do nothing",
    );
    assert!(
        refusal_note(&reply)?.contains("animation"),
        "the note names the gate that is closed",
    );
    assert!(
        sprite_draft(&app)?.def().animation.is_none(),
        "the refused op created no animation",
    );
    Ok(())
}

#[test]
fn an_armor_stat_over_its_range_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Armor)?;
    set_field(
        &mut app,
        &mut client,
        "(field: Armor(Floor(part: Head, value: 9)))",
    )?;

    let reply = try_set_field(
        &mut app,
        &mut client,
        "(field: Armor(Floor(part: Head, value: 101)))",
    )?;
    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains("101"),
        "the detail names the value that was out of range, got `{detail}`",
    );
    assert_eq!(
        *armor_draft(&app)?.spec().head.floor,
        9,
        "the piece still holds the value the last write in range left",
    );
    Ok(())
}

#[test]
fn a_list_op_on_the_armor_tab_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Armor)?;
    let before = armor_draft(&app)?;

    let reply = try_list_op(&mut app, &mut client, "(list: SpriteFrames, op: Add)")?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        armor_draft(&app)?,
        before,
        "the refused op left the Armor draft exactly as it was",
    );
    Ok(())
}

#[test]
fn a_toggle_on_the_sprite_frames_list_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    set_field(&mut app, &mut client, "(field: Sprite(Animated(true)))")?;
    let before = sprite_draft(&app)?;

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: SpriteFrames, op: Toggle(EntrySide(East)))",
    )?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        sprite_draft(&app)?,
        before,
        "the refused op left the Sprite draft exactly as it was",
    );
    Ok(())
}

#[test]
fn a_reorder_on_the_attachment_effects_list_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Attachment)?;
    let before = attachment_draft(&app)?;

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: AttachmentEffects, op: MoveUp(0))",
    )?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        attachment_draft(&app)?,
        before,
        "the refused op left the Attachment draft exactly as it was",
    );
    Ok(())
}

#[test]
fn removing_the_last_sprite_frame_is_refused_bad_arguments() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    set_field(&mut app, &mut client, "(field: Sprite(Animated(true)))")?;
    let before = sprite_draft(&app)?;

    let reply = try_list_op(&mut app, &mut client, "(list: SpriteFrames, op: Remove(0))")?;
    bad_arguments_detail(&reply)?;
    assert_eq!(
        sprite_draft(&app)?,
        before,
        "an animation keeps at least one frame, so the refused op left the one it had",
    );
    Ok(())
}

#[test]
fn a_write_naming_another_forms_field_is_refused_with_the_open_tab_named() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;

    let reply = try_set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the Kind arm belongs to the Terrain form, so the Injury tab refuses it",
    );
    assert!(
        refusal_note(&reply)?.contains("Injury"),
        "the note names the tab that is open, so a client can see which form it reached",
    );
    assert_eq!(
        crate::setup::terrain_draft(&app)?.kind(),
        gdtf_content_editor::TerrainKindChoice::default(),
        "an Injury writer that falls through to the Terrain draft would write it here",
    );
    Ok(())
}

#[test]
fn a_field_arm_on_the_armor_tab_is_refused_with_the_armor_tab_named() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Armor)?;
    let before = crate::setup::field_draft(&app)?;

    let write = try_set_field(&mut app, &mut client, "(field: Field(Damage(6)))")?;
    assert_eq!(
        unavailable_code(&write)?,
        "WrongState",
        "the Damage arm belongs to the Field form, so the Armor tab refuses it",
    );
    assert!(
        refusal_note(&write)?.contains("Armor"),
        "the note names the tab that is open",
    );

    let list = try_list_op(
        &mut app,
        &mut client,
        "(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Flak)))",
    )?;
    bad_arguments_detail(&list)?;
    assert_eq!(
        crate::setup::field_draft(&app)?,
        before,
        "an Armor-tab writer that fell through to the Field draft would write it here",
    );
    Ok(())
}

#[test]
fn a_field_write_before_the_autoload_settles_is_refused_and_names_the_gate() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Field)?;
    let before = crate::setup::field_draft(&app)?;
    assert!(
        before.autoload_pending(),
        "a fresh Field draft has not settled its first-frame autoload, which is the gate this \
         case needs",
    );

    let write = try_set_field(&mut app, &mut client, "(field: Field(Damage(6)))")?;
    assert_eq!(
        unavailable_code(&write)?,
        "WrongState",
        "a write onto an unsettled draft would be seeded over by the form's own sync, so it is \
         refused rather than run",
    );
    assert!(
        refusal_note(&write)?.contains("autoload"),
        "the note names the gate that is closed",
    );

    let list = try_list_op(
        &mut app,
        &mut client,
        "(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Flak)))",
    )?;
    assert_eq!(unavailable_code(&list)?, "WrongState");
    assert_eq!(
        crate::setup::field_draft(&app)?,
        before,
        "both refused writes left the Field draft exactly as it was",
    );
    Ok(())
}

#[test]
fn a_terrain_field_on_the_gang_tab_is_refused_with_the_gang_tab_named() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    let before = crate::setup::terrain_draft(&app)?;

    let reply = try_set_field(&mut app, &mut client, "(field: Terrain(Kind(Emplacement)))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the Kind arm belongs to the Terrain form, so the Gang tab refuses it",
    );
    assert!(
        refusal_note(&reply)?.contains("Gang"),
        "the note names the tab that is open",
    );
    assert_eq!(
        crate::setup::terrain_draft(&app)?,
        before,
        "a Gang writer that fell through to the Terrain draft would write it here",
    );
    Ok(())
}

#[test]
fn a_terrain_list_on_the_gang_tab_is_refused_with_the_gang_tab_named() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    let before = crate::setup::terrain_draft(&app)?;

    let reply = try_list_op(
        &mut app,
        &mut client,
        "(list: EntrySides, op: Toggle(EntrySide(East)))",
    )?;
    assert_eq!(unavailable_code(&reply)?, "WrongState");
    assert!(
        refusal_note(&reply)?.contains("Gang"),
        "the note names the tab that is open",
    );
    assert_eq!(
        crate::setup::terrain_draft(&app)?,
        before,
        "a Gang list editor that fell through to the Terrain draft would edit it here",
    );
    Ok(())
}

#[test]
fn a_list_naming_another_forms_list_is_refused_with_the_open_tab_named() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::MeleeWeapon)?;
    let before = crate::setup::injury_draft(&app)?;

    let reply = try_list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    assert_eq!(unavailable_code(&reply)?, "WrongState");
    assert!(
        refusal_note(&reply)?.contains("MeleeWeapon"),
        "the note names the tab that is open",
    );
    assert_eq!(
        crate::setup::injury_draft(&app)?,
        before,
        "the refused op left the Injury draft exactly as it was",
    );
    Ok(())
}
