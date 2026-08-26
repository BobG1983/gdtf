//! The slot and fitted-attachment lists both weapon forms draw, and the refusals they share.

use gdtf_battle_sim::equipment::attachments::{
    AttachmentName, AttachmentRegistry, FittedAttachments, WeaponSlots,
};
use gdtf_qa_protocol::command::RefusalNote;

use crate::net_qa::{
    commands::write::form_fault::FormWriteFault,
    wire::{AttachmentKeyNet, EditorListMemberNet, EditorListNet, EditorListOpNet, WeaponSlotNet},
};

const NO_ATTACHMENT_REGISTRY: RefusalNote = RefusalNote::from_static(
    "the fitted-attachment rows read the attachment registry, and it is absent or empty, so the \
     form disables its own Add button",
);

/// The note a list that draws no reorder buttons refuses a move with.
pub(super) const NO_REORDER: &str = "this list draws no reorder buttons. The Sprite form's animation frames are the one list \
     that reorders";

/// The note a list authored by add, remove and rewrite refuses a toggle with.
pub(super) const NO_TOGGLE: &str =
    "this list is authored by adding, removing and rewriting rows, so it offers no toggle";

/// The member the operation must carry for the list it names.
pub(super) fn wrong_member(list: EditorListNet) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "that member does not belong to {list:?}, so it names no row the form draws"
    ))
}

/// The fault an index beyond the list's own length answers.
pub(super) fn past_the_end(list: EditorListNet, index: usize, held: usize) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "{index} is past the end of {list:?}, which holds {held}"
    ))
}

/// The declarations of a slot list, as the reply reads them back.
pub(super) fn slot_members(slots: &WeaponSlots) -> Vec<EditorListMemberNet> {
    slots
        .declarations()
        .iter()
        .map(|declaration| EditorListMemberNet::Slot(WeaponSlotNet::from_declaration(*declaration)))
        .collect()
}

/// The keys of a fitted-attachment list, as the reply reads them back.
pub(super) fn attachment_members(fitted: &FittedAttachments) -> Vec<EditorListMemberNet> {
    fitted
        .iter()
        .map(|key| EditorListMemberNet::Attachment(AttachmentKeyNet::from_key(key)))
        .collect()
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

/// Apply one operation to a slot list, through the sim's own setters.
pub(super) fn apply_to_slots(
    slots: &mut WeaponSlots,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    let held = slots.declarations().len();
    match op {
        EditorListOpNet::Add => {
            slots.add_slot();
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if slots.remove_slot(*index) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::SetAt(index, member) => {
            let EditorListMemberNet::Slot(declaration) = member else {
                return Err(wrong_member(list));
            };
            if slots.set_slot(
                *index,
                declaration.slot().to_slot(),
                declaration.capacity().to_capacity(),
            ) {
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

/// Apply one operation to a fitted-attachment list, against the registry the picker reads.
pub(super) fn apply_to_attachments(
    fitted: &mut FittedAttachments,
    registry: Option<&AttachmentRegistry>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    let held = fitted.len();
    match op {
        EditorListOpNet::Add => {
            let seed = seed_key(registry)?;
            fitted.add(seed);
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if fitted.remove(*index) {
                Ok(())
            } else {
                Err(past_the_end(list, *index, held))
            }
        }
        EditorListOpNet::SetAt(index, EditorListMemberNet::Attachment(named)) => {
            let key = known_key(registry, &named)?;
            if fitted.set(*index, key) {
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
