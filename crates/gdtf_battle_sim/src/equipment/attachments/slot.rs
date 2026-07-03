//! The **attachment-slot vocabulary** (GTW-554) — the closed [`AttachmentSlot`] enum, the
//! [`SlotCapacity`] count, and the [`WeaponSlots`] declaration a weapon authors. A weapon
//! declares WHICH slots it offers (and how many attachments each holds); an attachment item
//! declares the SINGLE slot it occupies ([`slot`](super::AttachmentSpec::slot)); the
//! [`fit gate`](super::attachment_fits) admits an item only into a declared slot with free
//! capacity. Class gating EMERGES from slot availability — there is NO ranged/melee tag on an
//! item (a `Counterweight` cannot fit a gun that offers no counterweight slot; a `Muzzle` can
//! cannot fit a chainblade).

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// The **closed slot vocabulary** (GTW-554) — every mount point an attachment item can
/// occupy. A weapon offers a subset of these (its authored
/// [`slots`](crate::weapon::WeaponSpec::slots)); an attachment item occupies exactly ONE
/// (its authored [`slot`](super::AttachmentSpec::slot)).
///
/// CLOSED on purpose: the slot list is the fit-compatibility contract between weapons and
/// items, so it is a Rust enum (a typo'd slot fails to parse), not an open string. Ranged
/// weapons typically offer `Muzzle` / `Sight` / `Rail`; melee weapons offer `Counterweight` /
/// `Pommel` — but the CLASS gating emerges from which slots a weapon declares, never from a
/// class tag on the item.
///
/// Derives [`Serialize`] + [`Deserialize`] (RON round-trip stable — authored as the bare
/// variant name, e.g. `slot: Muzzle`), and `Hash`/`Eq`/`Copy` so occupancy bookkeeping can
/// count and compare slots cheaply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentSlot {
    /// The muzzle thread — suppressors, chokes, bore devices (a shot-exit mount).
    Muzzle,
    /// The sight dovetail — optics that concentrate the in-cone draw (Aim).
    Sight,
    /// The utility rail — braces, drums, magazines, and jury-rigged mods (the
    /// general-purpose mount, commonly multi-capacity).
    Rail,
    /// The melee counterweight socket — balance weights that change how a blade lands.
    Counterweight,
    /// The melee pommel — grip-end fittings on a hilted weapon.
    Pommel,
}

/// How many attachments ONE declared slot holds (GTW-554) — the per-slot capacity a weapon
/// authors next to each offered [`AttachmentSlot`] (e.g. `(Rail, 3)` — a long rail hosting
/// three mods).
///
/// A capacity newtype over `u8` (no-bare-types: a capacity is a domain value, never a bare
/// integer). Private inner + derived [`Deref`]; `#[serde(transparent)]` so it authors as the
/// bare RON scalar in the weapon's `slots:` pair list. The fit gate rejects an item once a
/// slot's occupied count reaches this capacity — cleanly, never by panic or eviction.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SlotCapacity(u8);

impl SlotCapacity {
    /// Build a per-slot capacity from its raw count.
    #[must_use]
    pub const fn new(capacity: u8) -> Self {
        Self(capacity)
    }
}

/// The **attachment slots a weapon offers** (GTW-554) — the authored
/// `slots: [(Muzzle, 1), (Sight, 1), (Rail, 3)]` list of a `.weapon.ron` /
/// `.melee_weapon.ron`, pairing each offered [`AttachmentSlot`] with its [`SlotCapacity`].
///
/// A named newtype over the declaration list (no-bare-types: a weapon's slot loadout is a
/// domain value, not a bare `Vec`). Private inner with a lookup accessor (the
/// [`AttachmentRegistry`](super::AttachmentRegistry) precedent — it answers a CAPACITY
/// question, not a raw-list question, so no derived `Deref`). `#[serde(transparent)]` so the
/// RON field authors the bare pair list. [`Default`] is the EMPTY declaration — a weapon that
/// authors no `slots:` field offers no slots, so NO attachment fits it (fail-closed; a thrown
/// grenade or a bare fist takes no fittings).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponSlots(Vec<(AttachmentSlot, SlotCapacity)>);

impl WeaponSlots {
    /// Build a slot declaration from its `(slot, capacity)` pairs — the shape the authored
    /// RON list deserializes into.
    #[must_use]
    pub const fn new(declarations: Vec<(AttachmentSlot, SlotCapacity)>) -> Self {
        Self(declarations)
    }

    /// The declared [`SlotCapacity`] for `slot`, or [`None`] when the weapon does not offer
    /// that slot — the fit gate's availability lookup. If a slot is (erroneously) declared
    /// twice, the FIRST declaration wins (documented authoring-error behavior — never a
    /// panic).
    #[must_use]
    pub fn capacity(&self, slot: AttachmentSlot) -> Option<SlotCapacity> {
        self.0
            .iter()
            .find(|(declared, _)| *declared == slot)
            .map(|&(_, capacity)| capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::{AttachmentSlot, SlotCapacity, WeaponSlots};

    /// Every closed-vocabulary slot round-trips through RON (Serialize → Deserialize →
    /// identity) — the C1 stability contract for authored content.
    #[test]
    fn every_slot_round_trips_through_ron() {
        for slot in [
            AttachmentSlot::Muzzle,
            AttachmentSlot::Sight,
            AttachmentSlot::Rail,
            AttachmentSlot::Counterweight,
            AttachmentSlot::Pommel,
        ] {
            let Ok(ron) = ron::ser::to_string(&slot) else {
                unreachable!("an AttachmentSlot serializes to RON");
            };
            let Ok(back) = ron::de::from_str::<AttachmentSlot>(&ron) else {
                unreachable!("the serialized slot `{ron}` parses back");
            };
            assert_eq!(back, slot, "the slot survives a RON round-trip");
        }
    }

    /// A `SlotCapacity` authors as the bare RON scalar (`#[serde(transparent)]`) and
    /// round-trips.
    #[test]
    fn slot_capacity_is_a_transparent_scalar() {
        let Ok(capacity) = ron::de::from_str::<SlotCapacity>("3") else {
            unreachable!("a bare scalar parses as a SlotCapacity");
        };
        assert_eq!(*capacity, 3, "the transparent scalar carries its count");
        let Ok(ron) = ron::ser::to_string(&capacity) else {
            unreachable!("a SlotCapacity serializes");
        };
        assert_eq!(ron, "3", "it serializes back to the bare scalar");
    }

    /// A weapon's `slots:` list parses from the authored pair form and answers capacity
    /// lookups; an undeclared slot answers `None` (the weapon does not offer it).
    #[test]
    fn weapon_slots_parse_and_answer_capacity() {
        let Ok(slots) = ron::de::from_str::<WeaponSlots>("[(Muzzle, 1), (Sight, 1), (Rail, 3)]")
        else {
            unreachable!("the authored slots pair-list parses");
        };
        assert_eq!(
            slots.capacity(AttachmentSlot::Rail),
            Some(SlotCapacity::new(3)),
            "a declared slot answers its authored capacity"
        );
        assert_eq!(
            slots.capacity(AttachmentSlot::Counterweight),
            None,
            "an undeclared slot answers None (not offered)"
        );
    }

    /// An omitted `slots:` field defaults to the EMPTY declaration — no slot is offered, so
    /// nothing fits (fail-closed).
    #[test]
    fn default_offers_no_slots() {
        let slots = WeaponSlots::default();
        assert_eq!(
            slots.capacity(AttachmentSlot::Muzzle),
            None,
            "the empty default offers no slots"
        );
    }

    /// A (mistakenly) duplicated declaration resolves to the FIRST entry — a documented
    /// authoring-error fallback, never a panic.
    #[test]
    fn duplicate_declaration_first_wins() {
        let slots = WeaponSlots::new(vec![
            (AttachmentSlot::Rail, SlotCapacity::new(2)),
            (AttachmentSlot::Rail, SlotCapacity::new(5)),
        ]);
        assert_eq!(
            slots.capacity(AttachmentSlot::Rail),
            Some(SlotCapacity::new(2)),
            "the first declaration wins on a duplicate"
        );
    }
}
