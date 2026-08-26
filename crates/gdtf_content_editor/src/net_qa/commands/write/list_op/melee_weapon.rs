//! The Melee Weapon draft's three lists: fight modes, slot declarations and fitted keys.

use gdtf_battle_sim::equipment::attachments::AttachmentRegistry;
use gdtf_qa_protocol::command::RefusalNote;

use super::shared::{
    NO_REORDER, NO_TOGGLE, apply_to_attachments, apply_to_slots, attachment_members, past_the_end,
    slot_members, wrong_member,
};
use crate::{
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorListMemberNet, EditorListNet, EditorListOpNet, FightModeSpecNet},
    },
};

const KEEPS_ONE_MODE: RefusalNote = RefusalNote::from_static(
    "a melee weapon keeps at least one fight mode, so the form disables its own Remove button \
     while one remains",
);

/// The members of the named list, as the reply reads them back.
pub(super) fn members(draft: &MeleeWeaponDraft, list: EditorListNet) -> Vec<EditorListMemberNet> {
    match list {
        EditorListNet::MeleeWeaponSlots => slot_members(&draft.spec().slots),
        EditorListNet::MeleeWeaponAttachments => attachment_members(&draft.spec().attachments),
        EditorListNet::MeleeWeaponFightModes => draft
            .fight_modes()
            .iter()
            .map(|mode| EditorListMemberNet::FightMode(FightModeSpecNet::from_spec(*mode)))
            .collect(),
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes => Vec::new(),
    }
}

fn fight_modes(draft: &mut MeleeWeaponDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let list = EditorListNet::MeleeWeaponFightModes;
    match op {
        EditorListOpNet::Add => {
            draft.add_fight_mode();
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if !draft.can_remove_fight_mode() {
                return Err(FormWriteFault::Gated(KEEPS_ONE_MODE));
            }
            let held = draft.fight_modes().len();
            if draft.remove_fight_mode(*index) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::SetAt(index, member) => {
            let EditorListMemberNet::FightMode(spec) = member else {
                return Err(wrong_member(list));
            };
            let held = draft.fight_modes().len();
            if draft.set_fight_mode(*index, spec.to_spec()) {
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

/// Apply one operation to the fight modes, the slots, or the fitted attachments.
pub(super) fn apply(
    draft: &mut MeleeWeaponDraft,
    registry: Option<&AttachmentRegistry>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    match list {
        EditorListNet::MeleeWeaponSlots => apply_to_slots(&mut draft.spec_mut().slots, list, op),
        EditorListNet::MeleeWeaponAttachments => {
            apply_to_attachments(&mut draft.spec_mut().attachments, registry, list, op)
        }
        EditorListNet::MeleeWeaponFightModes => fight_modes(draft, op),
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes => Err(FormWriteFault::ForeignArm),
    }
}
