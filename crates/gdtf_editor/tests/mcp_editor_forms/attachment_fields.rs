use gdtf_battle_sim::{
    effects::attachments::AttachmentEffect,
    equipment::attachments::AttachmentSlot,
    weapon::{AoeRange, ConeHalfAngle, HitType, ModeKind},
};
use gdtf_editor::EditorMode;

use crate::{
    rows::{AttachmentFieldRow, FieldRow},
    setup::{attachment_draft, form_tab_app_and_client, list_op, set_field},
    support::TestResult,
    values::{EffectRow, FireModeRow, HitTypeRow, ModeKindRow, SlotRow},
};

/// The fire mode this suite writes, all five fields the form's own row edits.
const A_FIRE_MODE: &str = "(field: Attachment(Effect(index: 0, effect: GainFireMode((kind: \
                           Burst, cone_mult: 1.5, tu_percent: 0.25, shots: 3, hit_type: \
                           Cone(range: 4, angle: 30.0))))))";

#[test]
fn every_attachment_field_arm_writes_the_draft_the_form_would_write() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Attachment)?;

    let named = set_field(
        &mut app,
        &mut client,
        "(field: Attachment(Name(\"red dot\")))",
    )?;
    assert_eq!(
        named.field,
        FieldRow::Attachment(AttachmentFieldRow::Name("red dot".to_owned())),
    );
    assert_eq!(
        attachment_draft(&app)?.name(),
        "red dot",
        "the world's own Attachment draft holds the name on the frame that answered",
    );

    let shown = set_field(
        &mut app,
        &mut client,
        "(field: Attachment(DisplayName(\"Red Dot Sight\")))",
    )?;
    assert_eq!(
        shown.field,
        FieldRow::Attachment(AttachmentFieldRow::DisplayName("Red Dot Sight".to_owned())),
    );

    let slotted = set_field(&mut app, &mut client, "(field: Attachment(Slot(Sight)))")?;
    assert_eq!(
        slotted.field,
        FieldRow::Attachment(AttachmentFieldRow::Slot(SlotRow::Sight)),
    );

    let draft = attachment_draft(&app)?;
    assert_eq!(
        draft.spec().display_name.as_str(),
        "Red Dot Sight",
        "the world's own draft holds the display name the write named",
    );
    assert_eq!(
        draft.spec().slot,
        AttachmentSlot::Sight,
        "the world's own draft holds the slot the write named",
    );

    list_op(&mut app, &mut client, "(list: AttachmentEffects, op: Add)")?;
    let written = set_field(&mut app, &mut client, A_FIRE_MODE)?;
    assert_eq!(
        written.field,
        FieldRow::Attachment(AttachmentFieldRow::Effect {
            index:  0,
            effect: EffectRow::GainFireMode(FireModeRow {
                kind:       ModeKindRow::Burst,
                cone_mult:  1.5,
                tu_percent: 0.25,
                shots:      3,
                hit_type:   HitTypeRow::Cone {
                    range: 4,
                    angle: 30.0,
                },
            }),
        }),
        "the reply reads the effect back off the draft, every field the form's row edits",
    );

    let written_draft = attachment_draft(&app)?;
    let Some(AttachmentEffect::GainFireMode(spec)) = written_draft.effects().first() else {
        return Err("the effect the write set is a GainFireMode in the world's own draft".into());
    };
    assert_eq!(
        (spec.kind, *spec.shots, spec.hit_type),
        (
            ModeKind::Burst,
            3,
            HitType::Cone {
                range: AoeRange::new(4),
                angle: ConeHalfAngle::new(30.0),
            }
        ),
        "the world's own draft holds the fire mode's kind, shots and hit geometry, so a wire \
         that dropped hit_type fails here",
    );
    Ok(())
}
