//! Slot capacity checks and pending-effect resolution.

use super::{AttachmentName, AttachmentRegistry, AttachmentSlot, WeaponSlots};
use crate::{effects::attachments::AttachmentEffect, weapon::PendingAttachments};

/// Why an attachment cannot occupy a weapon slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitRejection {
    /// Weapon does not offer this slot.
    UndeclaredSlot(AttachmentSlot),
    /// Slot is already full.
    SlotAtCapacity(AttachmentSlot),
}

/// Whether `slot` still has free capacity on this weapon.
///
/// # Errors
///
/// Returns [`FitRejection::UndeclaredSlot`] if the weapon never lists that slot,
/// or [`FitRejection::SlotAtCapacity`] if every slot of that kind is already used.
pub fn attachment_fits(
    slots: &WeaponSlots,
    fitted: &[AttachmentSlot],
    slot: AttachmentSlot,
) -> Result<(), FitRejection> {
    let Some(capacity) = slots.capacity(slot) else {
        return Err(FitRejection::UndeclaredSlot(slot));
    };
    let occupied = fitted.iter().filter(|occupied| **occupied == slot).count();
    if occupied >= usize::from(*capacity) {
        return Err(FitRejection::SlotAtCapacity(slot));
    }
    Ok(())
}

/// Resolve authored attachment keys into effects that fit the weapon's slots.
#[must_use]
pub fn resolve_pending_attachments(
    slots: &WeaponSlots,
    keys: &[AttachmentName],
    registry: Option<&AttachmentRegistry>,
) -> PendingAttachments {
    let Some(registry) = registry else {
        return PendingAttachments::default();
    };
    let mut fitted: Vec<AttachmentSlot> = Vec::new();
    let mut effects: Vec<AttachmentEffect> = Vec::new();
    for key in keys {
        let Some(spec) = registry.spec(key) else {
            continue;
        };
        if attachment_fits(slots, &fitted, spec.slot).is_err() {
            continue;
        }
        fitted.push(spec.slot);
        effects.extend(spec.effects.iter().cloned());
    }
    PendingAttachments::new(effects)
}
