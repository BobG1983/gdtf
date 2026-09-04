//! The Field form's own field arms, written through the draft its panels write.

use crate::{
    field_form::FieldDraft,
    mcp::{
        commands::write::form_fault::{FIELD_AUTOLOAD_PENDING, FormWriteFault},
        wire::{
            DamageTypeNet, EditorDraftNameNet, FieldDamageNet, FieldDurationNet, FieldFormFieldNet,
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
) -> Result<FieldFormFieldNet, FormWriteFault> {
    let Some(wanted) = duration.to_duration() else {
        return Err(FormWriteFault::bad(format!(
            "a field duration of Turns(0) never loads; the form's own turn drag starts at {}, \
             and this write is refused rather than clamped",
            FieldDraft::MIN_TURNS,
        )));
    };
    draft.set_duration(wanted);
    Ok(FieldFormFieldNet::Duration(
        FieldDurationNet::from_duration(draft.duration()),
    ))
}

/// Write one Field field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut FieldDraft,
    field: FieldFormFieldNet,
) -> Result<FieldFormFieldNet, FormWriteFault> {
    match field {
        FieldFormFieldNet::Name(name) => {
            settled(draft)?;
            draft.set_key((*name).clone());
            Ok(FieldFormFieldNet::Name(EditorDraftNameNet::new(
                draft.key(),
            )))
        }
        FieldFormFieldNet::Damage(damage) => {
            settled(draft)?;
            draft.set_damage(damage.to_damage());
            Ok(FieldFormFieldNet::Damage(FieldDamageNet::from_damage(
                draft.damage(),
            )))
        }
        FieldFormFieldNet::DamageType(damage_type) => {
            settled(draft)?;
            draft.set_damage_type(damage_type.to_damage_type());
            Ok(FieldFormFieldNet::DamageType(
                DamageTypeNet::from_damage_type(draft.damage_type()),
            ))
        }
        FieldFormFieldNet::Duration(duration) => {
            settled(draft)?;
            write_duration(draft, duration)
        }
    }
}
