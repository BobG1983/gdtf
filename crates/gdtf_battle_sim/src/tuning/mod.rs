//! its inner value and `#[serde(transparent)]` so it round-trips as a bare RON
mod band;
mod body_part;
mod combat_tuning;
mod cone;
mod cone_groups;
mod economy;
mod falls;
mod firing_arc;
mod matchup;
mod melee;
mod reaction;
mod severity;
mod slab;
mod stat_tuning;
mod visibility;
mod wounds;

#[cfg(test)]
mod test;

pub use band::{BandEdge, ProjectileBandEdges};
pub use body_part::{BodyPartWeight, BodyPartWeights};
pub use combat_tuning::CombatTuning;
pub use cone::{
    AimConeMult, AimHeightFrac, AimTuPremium, BraceContribution, ConcentrationCoeff,
    EmplacementStabilityBonus, MuzzleForwardOffset, MuzzleHeight, RecoilClimb, SilhouetteTop,
    StabilityCurveCoord, StanceContribution,
};
pub use cone_groups::{
    AimMode, BraceMinHeight, ConcentrationCoeffs, ConeStabilityTuning, MuzzleHeights,
    SilhouetteTops, StabilityCurve, StabilityCurvePoint, StabilityCurves, StanceStability,
};
pub use economy::{
    EnterEmplacementTu, ExitEmplacementTu, LinkTu, MoveCost, MoveCosts, OpenDoorTu, ShoveTu,
    StanceChangeTu, ThrowTu, TurnTu,
};
pub use falls::PerStoreyDamage;
pub use firing_arc::FiringArc;
pub use matchup::MatchupMultipliers;
pub use melee::{FightVariance, MeleeKMargin, MeleeMultMax, MeleeMultMin, MeleeTuning};
pub use reaction::{
    Interrupts, MayInterrupt, ReactionCap, ReactionCapBase, ReactionCapPerReactions, ReactionPMax,
    ReactionPMin, ReactionProbability, ReactionScore, ReactionTuning, ReactionsUsed,
    SuppressionRadius, SuppressionStabilityPenalty, clamp_probability, interrupt_probability,
    may_interrupt, reaction_cap, reaction_score, rolls_interrupt,
};
pub use severity::{
    DefenderLuckScale, PenDamageScale, RandomSpread, SeverityEdge, SeverityEdges, SeverityScaling,
    ShooterLuckScale, ToughnessMitigation,
};
pub use slab::{SlabDefaultHp, SlabDefaults};
pub use stat_tuning::{
    BottlePerMorale, FightWeights, GangerStatTuning, HpWeights, MoraleWeights, ReactionsWeights,
    ShootingWeights, StatWeight, TuBase, TuPerSpeed, WoundsPerHp,
};
pub use visibility::{ExploredDim, ViewRange};
pub use wounds::{BleedRate, ExecuteTu, StabilizeTu, WoundCost, WoundCosts};
