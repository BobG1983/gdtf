//! The Melee Weapon draft's three lists: fight modes, slot declarations and fitted keys.

use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry};
use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            AttachmentKeyNet, EditorListMemberNet, EditorListNet, EditorListOpNet,
            FightModeSpecNet, WeaponSlotNet,
        },
    },
};

const KEEPS_ONE_MODE: RefusalNote = RefusalNote::from_static(
    "a melee weapon keeps at least one fight mode, so the form disables its own Remove button \
     while one remains",
);

const NO_ATTACHMENT_REGISTRY: RefusalNote = RefusalNote::from_static(
    "the fitted-attachment rows read the attachment registry, and it is absent or empty, so the \
     form disables its own Add button",
);

const NO_REORDER: &str = "this list draws no reorder buttons. The Sprite form's animation frames are the one list \
     that reorders";

const NO_TOGGLE: &str =
    "this list is authored by adding, removing and rewriting rows, so it offers no toggle";

// The member the operation must carry for the list it names.
fn wrong_member(list: EditorListNet) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "that member does not belong to {list:?}, so it names no row the form draws"
    ))
}

fn past_the_end(list: EditorListNet, index: usize, held: usize) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "{index} is past the end of {list:?}, which holds {held}"
    ))
}

/// The members of the named list, as the reply reads them back.
pub(super) fn members(draft: &MeleeWeaponDraft, list: EditorListNet) -> Vec<EditorListMemberNet> {
    match list {
        EditorListNet::MeleeWeaponSlots => draft
            .spec()
            .slots
            .declarations()
            .iter()
            .map(|declaration| {
                EditorListMemberNet::Slot(WeaponSlotNet::from_declaration(*declaration))
            })
            .collect(),
        EditorListNet::MeleeWeaponAttachments => draft
            .spec()
            .attachments
            .iter()
            .map(|key| EditorListMemberNet::Attachment(AttachmentKeyNet::from_key(key)))
            .collect(),
        _ => draft
            .fight_modes()
            .iter()
            .map(|mode| EditorListMemberNet::FightMode(FightModeSpecNet::from_spec(*mode)))
            .collect(),
    }
}

// The first key the picker offers, which is what the form's Add button seeds.
fn seed_key(registry: Option<&AttachmentRegistry>) -> Result<AttachmentName, FormWriteFault> {
    let mut keys: Vec<&AttachmentName> =
        registry.map_or_else(Vec::new, |held| held.keys().collect());
    keys.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    match keys.first() {
        Some(first) => Ok((*first).clone()),
        None => Err(FormWriteFault::MissingModel(NO_ATTACHMENT_REGISTRY)),
    }
}

// The key the picker offers under this name, or the fault a name it lacks answers.
fn known_key(
    registry: Option<&AttachmentRegistry>,
    named: &AttachmentKeyNet,
) -> Result<AttachmentName, FormWriteFault> {
    let Some(registry) = registry else {
        return Err(FormWriteFault::MissingModel(NO_ATTACHMENT_REGISTRY));
    };
    let wanted = named.to_key();
    if registry.spec(&wanted).is_some() {
        Ok(wanted)
    } else {
        Err(FormWriteFault::bad(format!(
            "`{}` is not an attachment the registry holds, so the picker offers no such row",
            **named
        )))
    }
}

fn fight_modes(draft: &mut MeleeWeaponDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
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
                Err(past_the_end(
                    EditorListNet::MeleeWeaponFightModes,
                    *index,
                    held,
                ))
            }
        }
        EditorListOpNet::SetAt(index, member) => {
            let EditorListMemberNet::FightMode(spec) = member else {
                return Err(wrong_member(EditorListNet::MeleeWeaponFightModes));
            };
            let held = draft.fight_modes().len();
            if draft.set_fight_mode(*index, spec.to_spec()) {
                Ok(())
            } else {
                Err(past_the_end(
                    EditorListNet::MeleeWeaponFightModes,
                    *index,
                    held,
                ))
            }
        }
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(NO_TOGGLE.to_owned())),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => {
            Err(FormWriteFault::bad(NO_REORDER.to_owned()))
        }
    }
}

fn slots(draft: &mut MeleeWeaponDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let held = draft.spec().slots.declarations().len();
    let slots = &mut draft.spec_mut().slots;
    match op {
        EditorListOpNet::Add => {
            slots.add_slot();
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if slots.remove_slot(*index) {
                Ok(())
            } else {
                Err(past_the_end(EditorListNet::MeleeWeaponSlots, *index, held))
            }
        }
        EditorListOpNet::SetAt(index, member) => {
            let EditorListMemberNet::Slot(declaration) = member else {
                return Err(wrong_member(EditorListNet::MeleeWeaponSlots));
            };
            if slots.set_slot(
                *index,
                declaration.slot().to_slot(),
                declaration.capacity().to_capacity(),
            ) {
                Ok(())
            } else {
                Err(past_the_end(EditorListNet::MeleeWeaponSlots, *index, held))
            }
        }
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(NO_TOGGLE.to_owned())),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => {
            Err(FormWriteFault::bad(NO_REORDER.to_owned()))
        }
    }
}

fn attachments(
    draft: &mut MeleeWeaponDraft,
    registry: Option<&AttachmentRegistry>,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    let list = EditorListNet::MeleeWeaponAttachments;
    let held = draft.spec().attachments.len();
    match op {
        EditorListOpNet::Add => {
            let seed = seed_key(registry)?;
            draft.spec_mut().attachments.add(seed);
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if draft.spec_mut().attachments.remove(*index) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::SetAt(index, EditorListMemberNet::Attachment(named)) => {
            let key = known_key(registry, &named)?;
            if draft.spec_mut().attachments.set(*index, key) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::SetAt(..) => Err(wrong_member(list)),
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
        EditorListNet::MeleeWeaponSlots => slots(draft, op),
        EditorListNet::MeleeWeaponAttachments => attachments(draft, registry, op),
        _ => fight_modes(draft, op),
    }
}
