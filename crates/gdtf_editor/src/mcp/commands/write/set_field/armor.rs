//! The Armor form's own field arms, written through the draft the pieces grid writes.

use crate::{
    armor_form::ArmorDraft,
    mcp::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            ArmorFieldNet, ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet,
            ArmorTypeNet, BodyPartNet, EditorDraftNameNet,
        },
    },
};

// The floor the piece is set to, refused when the drag could not have reached it.
fn checked_floor(value: ArmorFloorNet) -> Result<ArmorFloorNet, FormWriteFault> {
    if ArmorDraft::STAT_RANGE.contains(&*value) {
        Ok(value)
    } else {
        Err(FormWriteFault::bad(format!(
            "{} is outside {:?}, the range the piece's floor, protection and hardness inputs \
             offer",
            *value,
            ArmorDraft::STAT_RANGE
        )))
    }
}

// The protection the piece is set to, refused when the drag could not have reached it.
fn checked_protection(value: ArmorProtectionNet) -> Result<ArmorProtectionNet, FormWriteFault> {
    if ArmorDraft::STAT_RANGE.contains(&*value) {
        Ok(value)
    } else {
        Err(FormWriteFault::bad(format!(
            "{} is outside {:?}, the range the piece's floor, protection and hardness inputs \
             offer",
            *value,
            ArmorDraft::STAT_RANGE
        )))
    }
}

// The hardness the piece is set to, refused when the drag could not have reached it.
fn checked_hardness(value: ArmorHardnessNet) -> Result<ArmorHardnessNet, FormWriteFault> {
    if ArmorDraft::STAT_RANGE.contains(&*value) {
        Ok(value)
    } else {
        Err(FormWriteFault::bad(format!(
            "{} is outside {:?}, the range the piece's floor, protection and hardness inputs \
             offer",
            *value,
            ArmorDraft::STAT_RANGE
        )))
    }
}

// The integrity the piece is set to, refused when the drag could not have reached it.
fn checked_integrity(value: ArmorIntegrityNet) -> Result<ArmorIntegrityNet, FormWriteFault> {
    if ArmorDraft::INTEGRITY_RANGE.contains(&*value) {
        Ok(value)
    } else {
        Err(FormWriteFault::bad(format!(
            "{} is outside {:?}, the range the piece's integrity input offers",
            *value,
            ArmorDraft::INTEGRITY_RANGE
        )))
    }
}

/// Write one Armor field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut ArmorDraft,
    field: ArmorFieldNet,
) -> Result<ArmorFieldNet, FormWriteFault> {
    match field {
        ArmorFieldNet::Name(name) => {
            draft.set_name((*name).clone());
            Ok(ArmorFieldNet::Name(EditorDraftNameNet::new(draft.name())))
        }
        ArmorFieldNet::Floor { part, value } => {
            let value = checked_floor(value)?;
            let written = part.to_part();
            let piece = draft.piece_mut(written);
            piece.floor = value.to_floor();
            Ok(ArmorFieldNet::Floor {
                part:  BodyPartNet::from_part(written),
                value: ArmorFloorNet::from_floor(piece.floor),
            })
        }
        ArmorFieldNet::Protection { part, value } => {
            let value = checked_protection(value)?;
            let written = part.to_part();
            let piece = draft.piece_mut(written);
            piece.protection = value.to_protection();
            Ok(ArmorFieldNet::Protection {
                part:  BodyPartNet::from_part(written),
                value: ArmorProtectionNet::from_protection(piece.protection),
            })
        }
        ArmorFieldNet::Hardness { part, value } => {
            let value = checked_hardness(value)?;
            let written = part.to_part();
            let piece = draft.piece_mut(written);
            piece.hardness = value.to_hardness();
            Ok(ArmorFieldNet::Hardness {
                part:  BodyPartNet::from_part(written),
                value: ArmorHardnessNet::from_hardness(piece.hardness),
            })
        }
        ArmorFieldNet::Integrity { part, value } => {
            let value = checked_integrity(value)?;
            let written = part.to_part();
            let piece = draft.piece_mut(written);
            piece.integrity = value.to_integrity();
            Ok(ArmorFieldNet::Integrity {
                part:  BodyPartNet::from_part(written),
                value: ArmorIntegrityNet::from_integrity(piece.integrity),
            })
        }
        ArmorFieldNet::Type { part, value } => {
            let written = part.to_part();
            let piece = draft.piece_mut(written);
            piece.armor_type = value.to_type();
            Ok(ArmorFieldNet::Type {
                part:  BodyPartNet::from_part(written),
                value: ArmorTypeNet::from_type(piece.armor_type),
            })
        }
    }
}
