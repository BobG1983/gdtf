use super::{AttachmentName, AttachmentRegistry, AttachmentSlot, WeaponSlots};
use crate::{effects::attachments::AttachmentEffect, weapon::PendingAttachments};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitRejection {
                UndeclaredSlot(AttachmentSlot),
            SlotAtCapacity(AttachmentSlot),
}

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

#[cfg(test)]
mod tests {
    use super::{FitRejection, attachment_fits, resolve_pending_attachments};
    use crate::{
        effects::attachments::{AimDelta, AttachmentEffect},
        equipment::attachments::{
            AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec, SlotCapacity,
            WeaponSlots,
        },
        weapon::{MagazineSize, WeaponName},
    };

            fn item(name: &str, slot: AttachmentSlot, effect: AttachmentEffect) -> AttachmentSpec {
        AttachmentSpec {
            display_name: WeaponName::new(name.to_owned()),
            slot,
            effects: vec![effect],
        }
    }

    fn key(name: &str) -> AttachmentName {
        AttachmentName::new(name.to_owned())
    }

            fn fixture_registry() -> AttachmentRegistry {
        AttachmentRegistry::new([
            (
                key("sight-a"),
                item(
                    "Sight A",
                    AttachmentSlot::Sight,
                    AttachmentEffect::Aim(AimDelta::new(0.4)),
                ),
            ),
            (
                key("sight-b"),
                item("Sight B", AttachmentSlot::Sight, AttachmentEffect::Silence),
            ),
            (
                key("muzzle-can"),
                item(
                    "Muzzle Can",
                    AttachmentSlot::Muzzle,
                    AttachmentEffect::Silence,
                ),
            ),
            (
                key("weight"),
                item(
                    "Counterweight",
                    AttachmentSlot::Counterweight,
                    AttachmentEffect::Damage(crate::weapon::WeaponDamage::new(4)),
                ),
            ),
            (
                key("rail-1"),
                item(
                    "Rail One",
                    AttachmentSlot::Rail,
                    AttachmentEffect::Aim(AimDelta::new(0.1)),
                ),
            ),
            (
                key("rail-2"),
                item("Rail Two", AttachmentSlot::Rail, AttachmentEffect::Silence),
            ),
            (
                key("rail-3"),
                item(
                    "Rail Three",
                    AttachmentSlot::Rail,
                    AttachmentEffect::ExtraAmmo(MagazineSize::new(10)),
                ),
            ),
        ])
    }

    fn sight_only() -> WeaponSlots {
        WeaponSlots::new(vec![(AttachmentSlot::Sight, SlotCapacity::new(1))])
    }

    fn rail(capacity: u8) -> WeaponSlots {
        WeaponSlots::new(vec![(AttachmentSlot::Rail, SlotCapacity::new(capacity))])
    }

        #[test]
    fn compatible_item_is_accepted() {
        let pending = resolve_pending_attachments(
            &sight_only(),
            &[key("sight-a")],
            Some(&fixture_registry()),
        );
        assert_eq!(
            pending.effects().len(),
            1,
            "the fitting Sight item contributes its one effect"
        );
    }

                #[test]
    fn wrong_slot_item_is_rejected() {
        let pending = resolve_pending_attachments(
            &sight_only(),
            &[key("muzzle-can"), key("weight")],
            Some(&fixture_registry()),
        );
        assert!(
            pending.effects().is_empty(),
            "neither the Muzzle nor the Counterweight item fits a Sight-only weapon"
        );
        assert_eq!(
            attachment_fits(&sight_only(), &[], AttachmentSlot::Counterweight),
            Err(FitRejection::UndeclaredSlot(AttachmentSlot::Counterweight)),
            "the gate names the undeclared slot"
        );
    }

            #[test]
    fn cap_one_slot_at_capacity_rejects() {
        let pending = resolve_pending_attachments(
            &sight_only(),
            &[key("sight-a"), key("sight-b")],
            Some(&fixture_registry()),
        );
        assert_eq!(
            pending.effects(),
            &[AttachmentEffect::Aim(AimDelta::new(0.4))],
            "only the FIRST Sight item's effect lands; the second is cleanly rejected"
        );
        assert_eq!(
            attachment_fits(
                &sight_only(),
                &[AttachmentSlot::Sight],
                AttachmentSlot::Sight
            ),
            Err(FitRejection::SlotAtCapacity(AttachmentSlot::Sight)),
            "the gate names the full slot"
        );
    }

            #[test]
    fn multi_cap_rail_accepts_up_to_capacity() {
        let pending = resolve_pending_attachments(
            &rail(3),
            &[key("rail-1"), key("rail-2"), key("rail-3")],
            Some(&fixture_registry()),
        );
        assert_eq!(
            pending.effects().len(),
            3,
            "all three Rail items fit a capacity-3 rail"
        );
    }

            #[test]
    fn rail_over_capacity_rejects_the_overflow() {
        let pending = resolve_pending_attachments(
            &rail(2),
            &[key("rail-1"), key("rail-2"), key("rail-3")],
            Some(&fixture_registry()),
        );
        assert_eq!(
            pending.effects(),
            &[
                AttachmentEffect::Aim(AimDelta::new(0.1)),
                AttachmentEffect::Silence,
            ],
            "the first two Rail items land; the third (over capacity) is cleanly rejected"
        );
    }

            #[test]
    fn missing_registry_and_missing_key_fail_safe() {
        let none = resolve_pending_attachments(&sight_only(), &[key("sight-a")], None);
        assert!(
            none.effects().is_empty(),
            "no registry resolves nothing (fail-safe)"
        );
        let skipped = resolve_pending_attachments(
            &sight_only(),
            &[key("no-such-item"), key("sight-a")],
            Some(&fixture_registry()),
        );
        assert_eq!(
            skipped.effects().len(),
            1,
            "an unresolved key is skipped without consuming the slot"
        );
    }
}
