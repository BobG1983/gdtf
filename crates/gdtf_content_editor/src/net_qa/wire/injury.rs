//! The Injury form's own field values and effect rows on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{BleedAmount, InjuryEffect, MovementCostFactor, StatDelta},
    severity::Severity,
};
use serde::{Deserialize, Serialize};

use super::stat_target::StatTargetNet;

/// The file stem the Injury form's key field holds.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct InjuryKeyNet(String);

impl InjuryKeyNet {
    /// Wrap a key a client sent or the draft holds.
    pub(in crate::net_qa) fn new(key: &str) -> Self {
        Self(key.to_owned())
    }
}

/// The text one of the injury's popup, log and inspect fields holds.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct InjuryTextNet(String);

impl InjuryTextNet {
    /// Wrap a line a client sent or the draft holds.
    pub(in crate::net_qa) fn new(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// Which body region an injury is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum InjuryCategoryNet {
    /// Head injuries.
    Head,
    /// Torso injuries.
    Torso,
    /// Arm injuries.
    Arm,
    /// Leg injuries.
    Leg,
}

impl InjuryCategoryNet {
    /// Mirror the sim's own category.
    pub(in crate::net_qa) const fn from_category(category: InjuryCategory) -> Self {
        match category {
            InjuryCategory::Head => Self::Head,
            InjuryCategory::Torso => Self::Torso,
            InjuryCategory::Arm => Self::Arm,
            InjuryCategory::Leg => Self::Leg,
        }
    }

    /// Read a client's category back as the sim's own.
    pub(in crate::net_qa) const fn to_category(self) -> InjuryCategory {
        match self {
            Self::Head => InjuryCategory::Head,
            Self::Torso => InjuryCategory::Torso,
            Self::Arm => InjuryCategory::Arm,
            Self::Leg => InjuryCategory::Leg,
        }
    }
}

/// The three ranks the injury table offers; `None` and `Fatal` have no arm here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum InjurySeverityNet {
    /// Light injury.
    Minor,
    /// Serious injury.
    Major,
    /// Life-threatening injury.
    Critical,
}

impl InjurySeverityNet {
    /// Mirror the sim's own rank, or nothing for a rank the form does not offer.
    pub(in crate::net_qa) const fn from_severity(severity: Severity) -> Option<Self> {
        match severity {
            Severity::Minor => Some(Self::Minor),
            Severity::Major => Some(Self::Major),
            Severity::Critical => Some(Self::Critical),
            Severity::None | Severity::Fatal => None,
        }
    }

    /// Read a client's rank back as the sim's own.
    pub(in crate::net_qa) const fn to_severity(self) -> Severity {
        match self {
            Self::Minor => Severity::Minor,
            Self::Major => Severity::Major,
            Self::Critical => Severity::Critical,
        }
    }
}

/// The amount a Modify effect moves its stat by.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct StatDeltaNet(i8);

impl StatDeltaNet {
    /// Wrap an amount.
    pub(in crate::net_qa) const fn new(amount: i8) -> Self {
        Self(amount)
    }
}

/// The hit points a Bleeding effect costs each turn.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct BleedAmountNet(u8);

impl BleedAmountNet {
    /// Wrap a bleed amount.
    pub(in crate::net_qa) const fn new(amount: u8) -> Self {
        Self(amount)
    }
}

/// The factor a movement-cost effect multiplies each step by.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct MovementCostFactorNet(f32);

impl MovementCostFactorNet {
    /// Wrap a factor.
    pub(in crate::net_qa) const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

/// One effect an injury applies, with the payload its own row edits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum InjuryEffectNet {
    /// Change a named stat.
    Modify {
        /// Which stat.
        stat:   StatTargetNet,
        /// The amount it moves by.
        amount: StatDeltaNet,
    },
    /// Accrue bleed damage per turn.
    Bleeding {
        /// The hit points it costs each turn.
        amount: BleedAmountNet,
    },
    /// Disable a hand.
    DisableHand,
    /// Multiply movement cost.
    MovementCostMul(MovementCostFactorNet),
}

impl InjuryEffectNet {
    /// Mirror the sim's own effect.
    pub(in crate::net_qa) const fn from_effect(effect: InjuryEffect) -> Self {
        match effect {
            InjuryEffect::Modify { stat, amount } => Self::Modify {
                stat:   StatTargetNet::from_target(stat),
                amount: StatDeltaNet::new(amount.raw()),
            },
            InjuryEffect::Bleeding { amount } => Self::Bleeding {
                amount: BleedAmountNet::new(amount.raw()),
            },
            InjuryEffect::DisableHand => Self::DisableHand,
            InjuryEffect::MovementCostMul(factor) => {
                Self::MovementCostMul(MovementCostFactorNet::new(factor.raw()))
            }
        }
    }

    /// Read a client's effect back as the sim's own.
    pub(in crate::net_qa) const fn to_effect(self) -> InjuryEffect {
        match self {
            Self::Modify { stat, amount } => InjuryEffect::Modify {
                stat:   stat.to_target(),
                amount: StatDelta::new(amount.0),
            },
            Self::Bleeding { amount } => InjuryEffect::Bleeding {
                amount: BleedAmount::new(amount.0),
            },
            Self::DisableHand => InjuryEffect::DisableHand,
            Self::MovementCostMul(factor) => {
                InjuryEffect::MovementCostMul(MovementCostFactor::new(factor.0))
            }
        }
    }
}
