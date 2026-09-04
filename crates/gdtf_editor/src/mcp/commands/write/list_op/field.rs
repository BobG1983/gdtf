//! The Field draft's immunity tick-box row: one toggle, and no other operation.

use crate::{
    field_form::FieldDraft,
    mcp::{
        commands::write::form_fault::{FIELD_AUTOLOAD_PENDING, FormWriteFault},
        wire::{ArmorTypeNet, EditorListMemberNet, EditorListOpNet},
    },
};

const TOGGLE_ONLY: &str = "the Field form draws its immune list as one tick box per ArmorType::ALL, so it has no \
     append, no position and no order. Toggle is the only operation it offers";

const WRONG_MEMBER: &str =
    "that member is not an armor type, so the toggle names no tick box the immune list draws";

/// The members of the immune list, in the order the reply reads them back.
pub(super) fn members(draft: &FieldDraft) -> Vec<EditorListMemberNet> {
    draft
        .immune_armor_types()
        .into_iter()
        .map(|armor_type| EditorListMemberNet::ImmuneArmorType(ArmorTypeNet::from_type(armor_type)))
        .collect()
}

/// Apply one operation to the immune-armor list.
pub(super) fn apply(draft: &mut FieldDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    if draft.autoload_pending() {
        return Err(FormWriteFault::Gated(FIELD_AUTOLOAD_PENDING));
    }
    let EditorListOpNet::Toggle(member) = op else {
        return Err(FormWriteFault::bad(TOGGLE_ONLY.to_owned()));
    };
    let EditorListMemberNet::ImmuneArmorType(armor_type) = member else {
        return Err(FormWriteFault::bad(WRONG_MEMBER.to_owned()));
    };
    draft.toggle_immune_armor_type(armor_type.to_type());
    Ok(())
}
