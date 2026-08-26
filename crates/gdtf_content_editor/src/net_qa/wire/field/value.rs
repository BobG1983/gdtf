//! The Field form's own values: the drain per tick and the lifetime a client sent.

use std::num::NonZeroU8;

use bevy::prelude::Deref;
use gdtf_battle_sim::effects::fields::{FieldDamage, FieldDuration, FieldTurns};
use serde::{Deserialize, Serialize};

/// The flat HP a field drains from an occupant each tick.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct FieldDamageNet(u16);

impl FieldDamageNet {
    /// Mirror the sim's own field damage.
    pub(in crate::net_qa) fn from_damage(damage: FieldDamage) -> Self {
        Self(*damage)
    }

    /// Read a client's field damage back as the sim's own.
    pub(in crate::net_qa) const fn to_damage(self) -> FieldDamage {
        FieldDamage::new(self.0)
    }
}

/// How many turns a placement of a field lasts, as a client sent it and unvalidated.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct FieldTurnsNet(u8);

impl FieldTurnsNet {
    /// Wrap a turn count a client sent or the placement holds.
    pub(in crate::net_qa) const fn new(turns: u8) -> Self {
        Self(turns)
    }

    /// Read the count back as the sim's own, or `None` when it is zero.
    pub(in crate::net_qa) const fn to_turns(self) -> Option<FieldTurns> {
        match NonZeroU8::new(self.0) {
            Some(count) => Some(FieldTurns::new(count)),
            None => None,
        }
    }
}

/// How long a placement of a field lasts, with the turn count a client sent unvalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum FieldDurationNet {
    /// A finite number of turns, which the sim requires to be at least one.
    Turns(FieldTurnsNet),
    /// Never expires.
    Permanent,
}

impl FieldDurationNet {
    /// Mirror the sim's own duration.
    pub(in crate::net_qa) fn from_duration(duration: FieldDuration) -> Self {
        match duration {
            FieldDuration::Turns(turns) => Self::Turns(FieldTurnsNet::new((*turns).get())),
            FieldDuration::Permanent => Self::Permanent,
        }
    }

    /// Read a client's duration back as the sim's own, or `None` for a zero turn count.
    pub(in crate::net_qa) const fn to_duration(self) -> Option<FieldDuration> {
        match self {
            Self::Permanent => Some(FieldDuration::Permanent),
            Self::Turns(count) => match count.to_turns() {
                Some(turns) => Some(FieldDuration::Turns(turns)),
                None => None,
            },
        }
    }
}
