//! The Weapon draft's four lists: fire modes, slots, fitted keys and on-death effects.

use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_sim::equipment::attachments::AttachmentRegistry;

use super::shared::{
    NO_REORDER, NO_TOGGLE, apply_to_attachments, apply_to_slots, attachment_members, no_slot_above,
    no_slot_below, past_the_end, slot_members, wrong_member,
};
use crate::{
    mcp::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            EditorListMemberNet, EditorListNet, EditorListOpNet, FireModeSpecNet, OnDeathEffectNet,
        },
    },
    weapon_form::{WeaponDraft, explode_template},
};

const KEEPS_ONE_MODE: RefusalNote = RefusalNote::from_static(
    "a weapon keeps at least one fire mode, so the form disables its own Remove mode button \
     while one remains",
);

const NOT_THROUGH_THE_LIST: &str = "one on-death effect is rewritten through the \
     `Weapon(OnDeathVariant(index: n, …))`, `Weapon(OnDeathHitType(index: n, …))`, \
     `Weapon(OnDeathDamage(index: n, …))`, `Weapon(OnDeathDamageType(index: n, …))` and \
     `Weapon(OnDeathField(index: n, …))` field arms, not through the list";

/// The members of the named list, as the reply reads them back.
pub(super) fn members(draft: &WeaponDraft, list: EditorListNet) -> Vec<EditorListMemberNet> {
    match list {
        EditorListNet::WeaponSlots => slot_members(&draft.spec().slots),
        EditorListNet::WeaponAttachments => attachment_members(&draft.spec().attachments),
        EditorListNet::WeaponFireModes => draft
            .fire_modes()
            .iter()
            .map(|mode| EditorListMemberNet::FireMode(FireModeSpecNet::from_spec(*mode)))
            .collect(),
        EditorListNet::WeaponOnDeathEffects => draft
            .spec()
            .on_death
            .iter()
            .map(|effect| EditorListMemberNet::OnDeathEffect(OnDeathEffectNet::from_effect(effect)))
            .collect(),
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::TerrainOnDeathEffects
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => Vec::new(),
    }
}

// Add, remove and reorder the on-death rows, the way the form's own buttons do.
fn on_death(draft: &mut WeaponDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let list = EditorListNet::WeaponOnDeathEffects;
    let effects = &mut draft.spec_mut().on_death;
    let held = effects.len();
    match op {
        EditorListOpNet::Add => {
            effects.push(explode_template());
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if *index < held {
                effects.remove(*index);
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::MoveUp(index) => {
            if *index > 0 && *index < held {
                effects.swap(*index - 1, *index);
                Ok(())
            } else {
                Err(no_slot_above(*index, held))
            }
        }
        EditorListOpNet::MoveDown(index) => {
            if *index + 1 < held {
                effects.swap(*index, *index + 1);
                Ok(())
            } else {
                Err(no_slot_below(*index, held))
            }
        }
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(NO_TOGGLE.to_owned())),
        EditorListOpNet::SetAt(..) => Err(FormWriteFault::bad(NOT_THROUGH_THE_LIST.to_owned())),
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

/// Apply one operation to the fire modes, the slots, the fitted attachments or the on-death list.
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
        EditorListNet::WeaponFireModes => fire_modes(draft, op),
        EditorListNet::WeaponOnDeathEffects => on_death(draft, op),
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::TerrainOnDeathEffects
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => Err(FormWriteFault::ForeignArm),
    }
}
