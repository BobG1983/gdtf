//! The Attachment form's own field arms, written through the draft its def panel writes.

use gdtf_battle_sim::weapon::WeaponName;

use crate::{
    attachment_form::AttachmentDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            AttachmentEffectNet, AttachmentSlotNet, EditorDraftNameNet, EditorFieldNet,
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
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::AttachmentName(name) => {
            draft.set_name((*name).clone());
            Ok(EditorFieldNet::AttachmentName(EditorDraftNameNet::new(
                draft.name(),
            )))
        }
        EditorFieldNet::AttachmentDisplayName(name) => {
            draft.spec_mut().display_name = WeaponName::new((*name).clone());
            Ok(EditorFieldNet::AttachmentDisplayName(
                EditorDraftNameNet::new(draft.spec().display_name.as_str()),
            ))
        }
        EditorFieldNet::AttachmentSlot(slot) => {
            draft.spec_mut().slot = slot.to_slot();
            Ok(EditorFieldNet::AttachmentSlot(
                AttachmentSlotNet::from_slot(draft.spec().slot),
            ))
        }
        EditorFieldNet::AttachmentEffect { index, effect } => {
            effect_at(draft, *index)?;
            draft.set_effect(*index, effect.to_effect());
            Ok(EditorFieldNet::AttachmentEffect {
                index:  EditorListIndexNet::new(*index),
                effect: effect_at(draft, *index)?,
            })
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}
