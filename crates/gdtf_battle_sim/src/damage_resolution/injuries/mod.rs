pub mod context;
pub mod def;
pub mod hands;
pub mod ledger;
pub mod registry;
pub mod roll;
pub mod rolled;
pub mod stat_target;
pub mod tables;
pub mod text;
pub mod weighting;

pub use context::DamageContext;
pub use def::{InjuryDef, PostHeal};
pub use hands::HandsAvailable;
pub use ledger::{BleedAfflicted, InflictedInjuries, StatDeltaLedger, StatDeltaSum};
pub use registry::InjuryRegistry;
pub use roll::roll_injury;
pub use rolled::{GainedInjury, RolledInjury};
pub use stat_target::{StatKind, StatTarget};
pub use tables::InjuryTables;
pub use text::{InjuryName, InspectText, LogText, PopupText};
pub use weighting::{InjuryWeight, InjuryWeighting, WeightedInjuryEntry, WeightedInjuryTable};

pub use crate::effects::injuries::{
    ApplyBleeding, ApplyDisableHand, ApplyInjuryEffect, ApplyModify, ApplyMovementCostMul,
    BleedAmount, HealError, InjuryEffect, LedgerAccumulators, MovementCostFactor, StatDelta,
};

#[cfg(test)]
mod test;
