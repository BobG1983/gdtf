//! Attachment slots and per-weapon capacity.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Where an attachment mounts on a weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentSlot {
    /// Muzzle device.
    Muzzle,
    /// Optic / sight.
    Sight,
    /// Rail accessory.
    Rail,
    /// Magazine well.
    Magazine,
    /// Counterweight.
    Counterweight,
    /// Pommel (melee).
    Pommel,
}

impl AttachmentSlot {
    /// All slots.
    pub const ALL: [Self; 6] = [
        Self::Muzzle,
        Self::Sight,
        Self::Rail,
        Self::Magazine,
        Self::Counterweight,
        Self::Pommel,
    ];
}

/// How many attachments of one slot type a weapon allows.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SlotCapacity(u8);

impl SlotCapacity {
    /// Wrap a capacity.
    #[must_use]
    pub const fn new(capacity: u8) -> Self {
        Self(capacity)
    }
}

/// Slot→capacity list a weapon declares.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponSlots(Vec<(AttachmentSlot, SlotCapacity)>);

impl WeaponSlots {
    /// From declarations.
    #[must_use]
    pub const fn new(declarations: Vec<(AttachmentSlot, SlotCapacity)>) -> Self {
        Self(declarations)
    }

    /// Capacity for a slot, if declared.
    #[must_use]
    pub fn capacity(&self, slot: AttachmentSlot) -> Option<SlotCapacity> {
        self.0
            .iter()
            .find(|(declared, _)| *declared == slot)
            .map(|&(_, capacity)| capacity)
    }

    /// All declarations.
    #[must_use]
    pub fn declarations(&self) -> &[(AttachmentSlot, SlotCapacity)] {
        &self.0
    }

    /// Declaration a newly added slot starts from.
    pub const DEFAULT_DECLARATION: (AttachmentSlot, SlotCapacity) =
        (AttachmentSlot::Muzzle, SlotCapacity::new(1));

    /// Append [`Self::DEFAULT_DECLARATION`].
    pub fn add_slot(&mut self) {
        self.0.push(Self::DEFAULT_DECLARATION);
    }

    /// Remove a declaration by index. Returns whether one was removed.
    pub fn remove_slot(&mut self, index: usize) -> bool {
        if index < self.0.len() {
            self.0.remove(index);
            true
        } else {
            false
        }
    }

    /// Rewrite a declaration by index. Returns whether one was written.
    pub fn set_slot(&mut self, index: usize, slot: AttachmentSlot, capacity: SlotCapacity) -> bool {
        match self.0.get_mut(index) {
            Some(declaration) => {
                *declaration = (slot, capacity);
                true
            }
            None => false,
        }
    }
}
