//! The Attachment form's own field arms, written through the draft its def panel writes.

use gdtf_battle_sim::weapon::WeaponName;

use crate::{
    attachment_form::AttachmentDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            AttachmentEffectNet, AttachmentFieldNet, AttachmentSlotNet, EditorDraftNameNet,
            EditorListIndexNet,
        },
    },
};

// The effect stored at one index, or the fault a past-the-end index answers.
fn effect_at(draft: &AttachmentDraft, index: usize) -> Result<AttachmentEffectNet, FormWriteFault> {
    match draft.effects().get(index) {
        Some(effect) => Ok(AttachmentEffectNet::from_effect(effect)),
        None => Err(FormWriteFault::bad(format!(
            "effect {index} is past the end of the attachment's effect list"
        ))),
    }
}

/// Write one Attachment field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut AttachmentDraft,
    field: AttachmentFieldNet,
) -> Result<AttachmentFieldNet, FormWriteFault> {
    match field {
        AttachmentFieldNet::Name(name) => {
            draft.set_name((*name).clone());
            Ok(AttachmentFieldNet::Name(EditorDraftNameNet::new(
                draft.name(),
            )))
        }
        AttachmentFieldNet::DisplayName(name) => {
            draft.spec_mut().display_name = WeaponName::new((*name).clone());
            Ok(AttachmentFieldNet::DisplayName(EditorDraftNameNet::new(
                draft.spec().display_name.as_str(),
            )))
        }
        AttachmentFieldNet::Slot(slot) => {
            draft.spec_mut().slot = slot.to_slot();
            Ok(AttachmentFieldNet::Slot(AttachmentSlotNet::from_slot(
                draft.spec().slot,
            )))
        }
        AttachmentFieldNet::Effect { index, effect } => {
            effect_at(draft, *index)?;
            draft.set_effect(*index, effect.to_effect());
            Ok(AttachmentFieldNet::Effect {
                index:  EditorListIndexNet::new(*index),
                effect: effect_at(draft, *index)?,
            })
        }
    }
}
