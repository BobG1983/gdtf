//! The Injury tab's weighting table on the wire: its damage source, buckets and rows.

use bevy::prelude::Deref;
use gdtf_battle_sim::injuries::{
    DamageContext, InjuryWeight, InjuryWeighting, WeightedInjuryEntry,
};
use serde::{Deserialize, Serialize};

use super::injury::{InjuryCategoryNet, InjuryKeyNet};

/// Which damage source a weighting table is authored for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum DamageContextNet {
    /// Shot or other ranged hit.
    Ranged,
    /// Melee strike.
    Melee,
    /// Fall damage.
    Fall,
}

impl DamageContextNet {
    /// Mirror the sim's own damage source.
    pub(in crate::mcp) const fn from_context(context: DamageContext) -> Self {
        match context {
            DamageContext::Ranged => Self::Ranged,
            DamageContext::Melee => Self::Melee,
            DamageContext::Fall => Self::Fall,
        }
    }

    /// Read a client's damage source back as the sim's own.
    pub(in crate::mcp) const fn to_context(self) -> DamageContext {
        match self {
            Self::Ranged => DamageContext::Ranged,
            Self::Melee => DamageContext::Melee,
            Self::Fall => DamageContext::Fall,
        }
    }
}

/// Which of the weighting table's three buckets a row sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum WeightingBucketNet {
    /// The Minor bucket.
    Minor,
    /// The Major bucket.
    Major,
    /// The Critical bucket.
    Critical,
}

/// The relative weight one weighting row carries.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct InjuryWeightNet(u32);

impl InjuryWeightNet {
    /// Mirror the sim's own weight.
    pub(in crate::mcp) fn from_weight(weight: InjuryWeight) -> Self {
        Self(*weight)
    }

    /// Read a client's weight back as the sim's own.
    pub(in crate::mcp) const fn to_weight(self) -> InjuryWeight {
        InjuryWeight::new(self.0)
    }
}

/// One row of a weighting bucket: the injury it names and the weight it carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) struct WeightingRowNet {
    injury: InjuryKeyNet,
    weight: InjuryWeightNet,
}

impl WeightingRowNet {
    /// Mirror the sim's own row.
    pub(in crate::mcp) fn from_entry(entry: &WeightedInjuryEntry) -> Self {
        Self {
            injury: InjuryKeyNet::new(entry.injury.as_str()),
            weight: InjuryWeightNet::from_weight(entry.weight),
        }
    }
}

/// The weighting table the Injury tab holds: its key, and all three buckets in draft order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::mcp) struct WeightingTableNet {
    category: InjuryCategoryNet,
    context:  DamageContextNet,
    minor:    Vec<WeightingRowNet>,
    major:    Vec<WeightingRowNet>,
    critical: Vec<WeightingRowNet>,
}

impl WeightingTableNet {
    /// Mirror the weighting a draft holds, bucket by bucket, in the order it holds them.
    pub(in crate::mcp) fn from_weighting(weighting: &InjuryWeighting) -> Self {
        Self {
            category: InjuryCategoryNet::from_category(weighting.category),
            context:  DamageContextNet::from_context(weighting.context),
            minor:    rows(&weighting.minor),
            major:    rows(&weighting.major),
            critical: rows(&weighting.critical),
        }
    }
}

// One bucket's rows, in the order the draft holds them.
fn rows(bucket: &[WeightedInjuryEntry]) -> Vec<WeightingRowNet> {
    bucket.iter().map(WeightingRowNet::from_entry).collect()
}
