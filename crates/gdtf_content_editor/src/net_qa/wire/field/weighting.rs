//! The Injury tab's weighting sub-tab fields, one arm per control its bucket rows draw.

use serde::{Deserialize, Serialize};

use crate::net_qa::wire::{
    injury::InjuryKeyNet,
    list::EditorListIndexNet,
    weighting::{InjuryWeightNet, WeightingBucketNet},
};

/// One field of the weighting draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum WeightingFieldNet {
    /// One weighting row's injury key, picked from the injury registry.
    RowInjury {
        /// Which bucket.
        bucket: WeightingBucketNet,
        /// Which row of it.
        index:  EditorListIndexNet,
        /// The injury key it is set to.
        injury: InjuryKeyNet,
    },
    /// One weighting row's relative weight.
    RowWeight {
        /// Which bucket.
        bucket: WeightingBucketNet,
        /// Which row of it.
        index:  EditorListIndexNet,
        /// The weight it is set to.
        weight: InjuryWeightNet,
    },
}
