//! The Melee Weapon form's own field values, fight modes, slots and fitted keys on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentSlot, SlotCapacity},
    weapon::{FightModeKind, FightModeSpec, Handedness, Reach, Shove, Strikes, TuCost},
};
use serde::{Deserialize, Serialize};

use super::attachment::AttachmentSlotNet;

/// How many hands a weapon takes, mirroring the two the form's combo offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum HandednessNet {
    /// One-handed.
    OneHanded,
    /// Two-handed.
    TwoHanded,
}

impl HandednessNet {
    /// Mirror the sim's own handedness.
    pub(in crate::mcp) const fn from_handedness(handedness: Handedness) -> Self {
        match handedness {
            Handedness::OneHanded => Self::OneHanded,
            Handedness::TwoHanded => Self::TwoHanded,
        }
    }

    /// Read a client's handedness back as the sim's own.
    pub(in crate::mcp) const fn to_handedness(self) -> Handedness {
        match self {
            Self::OneHanded => Handedness::OneHanded,
            Self::TwoHanded => Handedness::TwoHanded,
        }
    }
}

/// A weapon's strike range in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ReachNet(u16);

impl ReachNet {
    /// Wrap a reach, for a case that spells one a client could send.
    #[cfg(test)]
    pub(in crate::mcp) const fn new(reach: u16) -> Self {
        Self(reach)
    }

    /// Mirror the sim's own reach.
    pub(in crate::mcp) fn from_reach(reach: Reach) -> Self {
        Self(*reach)
    }
}

/// Whether a connecting hit knocks its target back.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct ShoveNet(bool);

impl ShoveNet {
    /// Wrap the flag a client sent or the spec holds.
    pub(in crate::mcp) const fn new(shove: bool) -> Self {
        Self(shove)
    }

    /// Read a client's flag back as the sim's own.
    pub(in crate::mcp) const fn to_shove(self) -> Shove {
        Shove::new(self.0)
    }
}

/// Swing or thrust, the two a fight-mode row offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum FightModeKindNet {
    /// Wide swing.
    Swing,
    /// Straight thrust.
    Thrust,
}

impl FightModeKindNet {
    /// Mirror the sim's own kind.
    pub(in crate::mcp) const fn from_kind(kind: FightModeKind) -> Self {
        match kind {
            FightModeKind::Swing => Self::Swing,
            FightModeKind::Thrust => Self::Thrust,
        }
    }

    /// Read a client's kind back as the sim's own.
    pub(in crate::mcp) const fn to_kind(self) -> FightModeKind {
        match self {
            Self::Swing => FightModeKind::Swing,
            Self::Thrust => FightModeKind::Thrust,
        }
    }
}

/// The time units one fight mode charges.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct TuCostNet(u16);

impl TuCostNet {
    /// Wrap a cost.
    pub(in crate::mcp) const fn new(tu: u16) -> Self {
        Self(tu)
    }
}

/// How many strikes one fight mode lands.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct StrikesNet(u16);

impl StrikesNet {
    /// Wrap a strike count.
    pub(in crate::mcp) const fn new(strikes: u16) -> Self {
        Self(strikes)
    }
}

/// One fight mode, all three fields the form's row edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) struct FightModeSpecNet {
    /// Swing or thrust.
    kind:    FightModeKindNet,
    /// The time units it charges.
    tu_cost: TuCostNet,
    /// How many strikes it lands.
    strikes: StrikesNet,
}

impl FightModeSpecNet {
    /// Build a mode, for a case that spells one a client could send.
    #[cfg(test)]
    pub(in crate::mcp) const fn new(
        kind: FightModeKindNet,
        tu_cost: TuCostNet,
        strikes: StrikesNet,
    ) -> Self {
        Self {
            kind,
            tu_cost,
            strikes,
        }
    }

    /// Mirror the sim's own mode.
    pub(in crate::mcp) fn from_spec(spec: FightModeSpec) -> Self {
        Self {
            kind:    FightModeKindNet::from_kind(spec.kind),
            tu_cost: TuCostNet::new(*spec.tu_cost),
            strikes: StrikesNet::new(*spec.strikes),
        }
    }

    /// Read a client's mode back as the sim's own.
    pub(in crate::mcp) const fn to_spec(self) -> FightModeSpec {
        FightModeSpec::new(
            self.kind.to_kind(),
            TuCost::new(self.tu_cost.0),
            Strikes::new(self.strikes.0),
        )
    }
}

/// How many attachments one declared slot allows.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct SlotCapacityNet(u8);

impl SlotCapacityNet {
    /// Wrap a capacity.
    pub(in crate::mcp) const fn new(capacity: u8) -> Self {
        Self(capacity)
    }

    /// Read a client's capacity back as the sim's own.
    pub(in crate::mcp) const fn to_capacity(self) -> SlotCapacity {
        SlotCapacity::new(self.0)
    }
}

/// One slot declaration, the pair the form's row edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) struct WeaponSlotNet {
    /// Where the attachment mounts.
    slot:     AttachmentSlotNet,
    /// How many that slot allows.
    capacity: SlotCapacityNet,
}

impl WeaponSlotNet {
    /// Build a declaration, for a case that spells one a client could send.
    #[cfg(test)]
    pub(in crate::mcp) const fn new(slot: AttachmentSlotNet, capacity: SlotCapacityNet) -> Self {
        Self { slot, capacity }
    }

    /// Mirror the sim's own declaration.
    pub(in crate::mcp) fn from_declaration(declaration: (AttachmentSlot, SlotCapacity)) -> Self {
        Self {
            slot:     AttachmentSlotNet::from_slot(declaration.0),
            capacity: SlotCapacityNet::new(*declaration.1),
        }
    }

    /// Where the attachment mounts.
    pub(in crate::mcp) const fn slot(self) -> AttachmentSlotNet {
        self.slot
    }

    /// How many that slot allows.
    pub(in crate::mcp) const fn capacity(self) -> SlotCapacityNet {
        self.capacity
    }
}

/// The registry key one fitted attachment names.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct AttachmentKeyNet(String);

impl AttachmentKeyNet {
    /// Mirror the sim's own key.
    pub(in crate::mcp) fn from_key(key: &AttachmentName) -> Self {
        Self(key.as_str().to_owned())
    }

    /// Read a client's key back as the sim's own.
    pub(in crate::mcp) fn to_key(&self) -> AttachmentName {
        AttachmentName::new(self.0.clone())
    }
}
