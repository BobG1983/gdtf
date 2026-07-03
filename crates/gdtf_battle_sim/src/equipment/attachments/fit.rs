//! The **slot fit gate** (GTW-554) — the mechanics that decide whether an attachment item
//! FITS a weapon: [`attachment_fits`] (the single-item gate) and
//! [`resolve_pending_attachments`] (the setup-time resolution BOTH weapon-spawn paths call —
//! `setup_battle`'s wielded spawn and the emplacement mounted-weapon spawn).
//!
//! An item fits ONLY if the weapon DECLARES the item's slot
//! ([`WeaponSlots::capacity`](super::WeaponSlots::capacity) answers) AND that slot has free
//! capacity (occupied < capacity). At capacity the item is CLEANLY REJECTED — no auto-evict
//! (the swap UX is a deliberate `HiveScape` follow-up), no panic. Class gating EMERGES from the
//! declarations: no ranged/melee tag exists on an item — a `Counterweight` simply finds no
//! slot on a gun that offers none.

use super::{AttachmentName, AttachmentRegistry, AttachmentSlot, WeaponSlots};
use crate::{effects::attachments::AttachmentEffect, weapon::PendingAttachments};

/// Why an attachment item does NOT fit a weapon (GTW-554) — the clean-rejection reasons the
/// [`attachment_fits`] gate answers. A rejection is a normal, non-fatal outcome: the
/// resolution skips the item (applying nothing), never panics, and never evicts a fitted
/// item to make room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitRejection {
    /// The weapon declares no such slot — the item's mount point does not exist on this
    /// weapon (how class gating emerges: a `Counterweight` on a gun, a `Muzzle` can on a
    /// chainblade).
    UndeclaredSlot(AttachmentSlot),
    /// The slot is declared but FULL — its occupied count has reached the authored
    /// [`SlotCapacity`](super::SlotCapacity) (swap/evict UX is deliberately deferred).
    SlotAtCapacity(AttachmentSlot),
}

/// Does an item occupying `slot` fit a weapon declaring `slots`, given the slots its
/// already-fitted items occupy (`fitted`, one entry per fitted item)?
///
/// The GTW-554 fit rule, verbatim: the weapon must DECLARE the slot AND the slot must have
/// free capacity (`occupied < capacity`). Returns the typed [`FitRejection`] otherwise —
/// callers skip the item (clean rejection), they never panic or evict.
///
/// # Errors
///
/// - [`FitRejection::UndeclaredSlot`] — the weapon offers no such slot.
/// - [`FitRejection::SlotAtCapacity`] — every declared position is already occupied (a
///   declared capacity of `0` therefore fits nothing).
pub fn attachment_fits(
    slots: &WeaponSlots,
    fitted: &[AttachmentSlot],
    slot: AttachmentSlot,
) -> Result<(), FitRejection> {
    let Some(capacity) = slots.capacity(slot) else {
        return Err(FitRejection::UndeclaredSlot(slot));
    };
    // The occupied count is a local index into a list we own (no-bare-types carve-out).
    let occupied = fitted.iter().filter(|occupied| **occupied == slot).count();
    if occupied >= usize::from(*capacity) {
        return Err(FitRejection::SlotAtCapacity(slot));
    }
    Ok(())
}

/// Resolve a weapon's authored attachment KEYS against the [`AttachmentRegistry`] — gated by
/// the GTW-554 slot fit — into the flat list of
/// [`AttachmentEffect`](crate::weapon::AttachmentEffect)s to apply, wrapped in a
/// [`PendingAttachments`] marker for the spawned weapon entity.
///
/// Keys are processed in authored order. Each resolved item must FIT ([`attachment_fits`]):
/// its [`slot`](super::AttachmentSpec::slot) must be declared in the weapon's `slots` and
/// have free capacity — a fitting item occupies one position and contributes its `effects`
/// (cloned, in authored order); a non-fitting item is CLEANLY REJECTED (skipped — it occupies
/// nothing and applies nothing, never a panic or an eviction). A missing registry
/// (`registry == None`, a test fixture without the loaded catalog) or an unresolved key
/// contributes NO effects and consumes NO capacity — the fail-safe every content-registry
/// resolution shares. An empty result is the identity: the post-spawn
/// [`apply_pending_attachments`](super::apply_pending_attachments) system no-ops.
///
/// This is the ONE resolution seam — `setup_battle`'s ranged AND melee wielded spawns and the
/// emplacement mounted-weapon spawn all resolve through it, so the fit rule cannot drift
/// between paths.
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
        // A missing key fails safe: nothing applied, no capacity consumed.
        let Some(spec) = registry.spec(key) else {
            continue;
        };
        // The fit gate: an undeclared slot or a full slot is a CLEAN rejection — skip.
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

    /// An item spec occupying `slot` with one discriminating `effect` (arbitrary magnitudes —
    /// never shipped content).
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

    /// A registry of distinct-slot fixture items: one Sight, one Muzzle, one Counterweight,
    /// and three Rail items (each with a discriminating effect).
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

    /// (a) A compatible item — declared slot, free capacity — is ACCEPTED: its effects land.
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

    /// (b) A wrong-slot item — the weapon never declares its slot — is CLEANLY REJECTED (and
    /// the gate names the reason). Class gating emerges here: a Counterweight cannot fit a
    /// weapon whose declarations are ranged-style.
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

    /// (c) A capacity-1 slot at capacity REJECTS the next item — the first Sight fits, the
    /// second is skipped (no eviction, no panic).
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

    /// (d) A multi-capacity Rail accepts UP TO its capacity — all three items fit a
    /// capacity-3 rail.
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

    /// (e) One OVER capacity is rejected — a capacity-2 rail fits the first two items and
    /// cleanly rejects the third (its effect never lands).
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

    /// A missing registry or an unresolved key fails safe: nothing applied — and an
    /// unresolved key consumes NO capacity (the later fitting item still lands).
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
