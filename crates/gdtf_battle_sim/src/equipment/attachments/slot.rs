//! [`SlotCapacity`] count, and the [`WeaponSlots`] declaration a weapon authors. A weapon
use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentSlot {
        Muzzle,
        Sight,
                Rail,
                        Magazine,
        Counterweight,
        Pommel,
}

impl AttachmentSlot {
                pub const ALL: [Self; 6] = [
        Self::Muzzle,
        Self::Sight,
        Self::Rail,
        Self::Magazine,
        Self::Counterweight,
        Self::Pommel,
    ];
}

/// integer). Private inner + derived [`Deref`]; `#[serde(transparent)]` so it authors as the
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SlotCapacity(u8);

impl SlotCapacity {
        #[must_use]
    pub const fn new(capacity: u8) -> Self {
        Self(capacity)
    }
}

/// question, not a raw-list question, so no derived `Deref`). `#[serde(transparent)]` so the
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponSlots(Vec<(AttachmentSlot, SlotCapacity)>);

impl WeaponSlots {
            #[must_use]
    pub const fn new(declarations: Vec<(AttachmentSlot, SlotCapacity)>) -> Self {
        Self(declarations)
    }

                    #[must_use]
    pub fn capacity(&self, slot: AttachmentSlot) -> Option<SlotCapacity> {
        self.0
            .iter()
            .find(|(declared, _)| *declared == slot)
            .map(|&(_, capacity)| capacity)
    }

                    #[must_use]
    pub fn declarations(&self) -> &[(AttachmentSlot, SlotCapacity)] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{AttachmentSlot, SlotCapacity, WeaponSlots};

            #[test]
    fn every_slot_round_trips_through_ron() {
        for slot in [
            AttachmentSlot::Muzzle,
            AttachmentSlot::Sight,
            AttachmentSlot::Rail,
            AttachmentSlot::Magazine,
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

            #[test]
    fn default_offers_no_slots() {
        let slots = WeaponSlots::default();
        assert_eq!(
            slots.capacity(AttachmentSlot::Muzzle),
            None,
            "the empty default offers no slots"
        );
    }

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
