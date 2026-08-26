//! The Field form's own field arms, written through the draft its panels write.

use crate::{
    field_form::FieldDraft,
    net_qa::{
        commands::write::form_fault::{FIELD_AUTOLOAD_PENDING, FormWriteFault},
        wire::{
            DamageTypeNet, EditorDraftNameNet, EditorFieldNet, FieldDamageNet, FieldDurationNet,
        },
    },
};

// The draft must have settled before a write lands, or the autoload would overwrite it.
const fn settled(draft: &FieldDraft) -> Result<(), FormWriteFault> {
    if draft.autoload_pending() {
        return Err(FormWriteFault::Gated(FIELD_AUTOLOAD_PENDING));
    }
    Ok(())
}

fn write_duration(
    draft: &mut FieldDraft,
    duration: FieldDurationNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    let Some(wanted) = duration.to_duration() else {
        return Err(FormWriteFault::bad(format!(
            "a field duration of Turns(0) never loads; the form's own turn drag starts at {}, \
             and this write is refused rather than clamped",
            FieldDraft::MIN_TURNS,
        )));
    };
    draft.set_duration(wanted);
    Ok(EditorFieldNet::FieldDuration(
        FieldDurationNet::from_duration(draft.duration()),
    ))
}

/// Write one Field field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut FieldDraft,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::FieldName(name) => {
            settled(draft)?;
            draft.set_key((*name).clone());
            Ok(EditorFieldNet::FieldName(EditorDraftNameNet::new(
                draft.key(),
            )))
        }
        EditorFieldNet::FieldDamage(damage) => {
            settled(draft)?;
            draft.set_damage(damage.to_damage());
            Ok(EditorFieldNet::FieldDamage(FieldDamageNet::from_damage(
                draft.damage(),
            )))
        }
        EditorFieldNet::FieldDamageType(damage_type) => {
            settled(draft)?;
            draft.set_damage_type(damage_type.to_damage_type());
            Ok(EditorFieldNet::FieldDamageType(
                DamageTypeNet::from_damage_type(draft.damage_type()),
            ))
        }
        EditorFieldNet::FieldDuration(duration) => {
            settled(draft)?;
            write_duration(draft, duration)
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}
