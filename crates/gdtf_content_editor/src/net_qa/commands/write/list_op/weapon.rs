//! The Weapon draft's three lists: fire modes, slot declarations and fitted keys.

use gdtf_battle_sim::equipment::attachments::AttachmentRegistry;
use gdtf_qa_protocol::command::RefusalNote;

use super::shared::{
    NO_REORDER, NO_TOGGLE, apply_to_attachments, apply_to_slots, attachment_members, past_the_end,
    slot_members, wrong_member,
};
use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorListMemberNet, EditorListNet, EditorListOpNet, FireModeSpecNet},
    },
    weapon_form::WeaponDraft,
};

const KEEPS_ONE_MODE: RefusalNote = RefusalNote::from_static(
    "a weapon keeps at least one fire mode, so the form disables its own Remove mode button \
     while one remains",
);

/// The members of the named list, as the reply reads them back.
pub(super) fn members(draft: &WeaponDraft, list: EditorListNet) -> Vec<EditorListMemberNet> {
    match list {
        EditorListNet::WeaponSlots => slot_members(&draft.spec().slots),
        EditorListNet::WeaponAttachments => attachment_members(&draft.spec().attachments),
        _ => draft
            .fire_modes()
            .iter()
            .map(|mode| EditorListMemberNet::FireMode(FireModeSpecNet::from_spec(*mode)))
            .collect(),
    }
}

fn fire_modes(draft: &mut WeaponDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let list = EditorListNet::WeaponFireModes;
    match op {
        EditorListOpNet::Add => {
            draft.add_fire_mode();
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if !draft.can_remove_fire_mode() {
                return Err(FormWriteFault::Gated(KEEPS_ONE_MODE));
            }
            let held = draft.fire_modes().len();
            if draft.remove_fire_mode(*index) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::SetAt(index, member) => {
            let EditorListMemberNet::FireMode(spec) = member else {
                return Err(wrong_member(list));
            };
            let held = draft.fire_modes().len();
            if draft.set_fire_mode(*index, spec.to_spec()) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(NO_TOGGLE.to_owned())),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => {
            Err(FormWriteFault::bad(NO_REORDER.to_owned()))
        }
    }
}

/// Apply one operation to the fire modes, the slots, or the fitted attachments.
pub(super) fn apply(
    draft: &mut WeaponDraft,
    registry: Option<&AttachmentRegistry>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    match list {
        EditorListNet::WeaponSlots => apply_to_slots(&mut draft.spec_mut().slots, list, op),
        EditorListNet::WeaponAttachments => {
            apply_to_attachments(&mut draft.spec_mut().attachments, registry, list, op)
        }
        _ => fire_modes(draft, op),
    }
}
