//! Combat tuning data — the single home for every balance coefficient.
//!
//! The equation *forms* live in code (`docs/combat/resolution.md`); every
//! *coefficient* is a default here, so balancing is a data edit, not a code
//! change ("Coefficients live in the combat-tuning data", resolution.md §"What's
//! pure math vs sim"). [`CombatTuning`] is a Bevy [`Resource`](bevy::prelude::Resource)
//! that deserializes from a `.ron` file, and **no numeric tuning literal lives
//! anywhere outside this module** — the [`crate::metric::MAX_LEVELS`]
//! coordinate-system constant is the only other named number.
//!
//! Every numeric leaf is a named newtype (no bare `f32`/`u16` field), per the
//! no-bare-types rule: each carries a derived [`Deref`](bevy::prelude::Deref) to
//! its inner value and `#[serde(transparent)]` so it round-trips as a bare RON
//! scalar.
//!
//! ## Module map (split by tuning DOMAIN, GTW-201 code-health wave)
//!
//! - [`band`] — the projectile clearance band edges (battle-space.md §"Banding").
//! - [`severity`] — the resolution.md §6 wound-severity scaling + bucket edges.
//! - [`body_part`] — the §4 body-part hit-location weights.
//! - [`wounds`] — the per-tier Wounds-budget costs, bleed-out rate, and from-Downed
//!   TU costs (E3.6 / E3.7 / E3.8).
//! - [`economy`] — the E4 TU economy: stance-change / turn / per-terrain move / per-link costs.
//! - [`cone`] — the §1 cone/stability/recoil/aim leaf coefficient newtypes.
//! - [`cone_groups`] — the §1 grouping structs + the [`ConeStabilityTuning`] bundle.
//! - [`matchup`] — the 7-type matchup multipliers (E3.2).
//! - [`firing_arc`] — the GTW-242 firing arc.
//! - [`visibility`] — the GTW-338 squad fog-of-war view range + explored dim.
//! - [`slab`] — the GTW-365 slab-defaults (uniform HP + armor a struck slab seeds to).
//! - [`reaction`] — the §8 reaction-fire layer: the GTW-466 tuning group
//!   ([`ReactionTuning`] + four leaves + pure functions [`reaction_cap`] /
//!   [`clamp_probability`]) and the GTW-467 deterministic opposed-check core
//!   (output newtypes [`ReactionScore`] / [`ReactionProbability`] /
//!   [`ReactionsUsed`] + pure functions [`reaction_score`] /
//!   [`interrupt_probability`] / [`rolls_interrupt`] / [`may_interrupt`]).
//! - [`melee`] — the §7 melee opposed-Fight layer: the GTW-506 tuning group
//!   ([`MeleeTuning`] + four leaves [`MeleeKMargin`] / [`MeleeMultMin`] /
//!   [`MeleeMultMax`] / [`FightVariance`]). The pure §7 functions that consume it
//!   ([`opposed_fight`](crate::melee::opposed_fight) /
//!   [`melee_damage_mult`](crate::melee::melee_damage_mult)) live in [`crate::melee`].
//! - [`combat_tuning`] — the top-level [`CombatTuning`] resource composing them all.
//! - [`stat_tuning`] — the GTW-384 [`GangerStatTuning`] resource: the attribute →
//!   computed-stat derivation weights / divisors / TU params (a SEPARATE store from
//!   `CombatTuning`, the user-directed split). Loaded from `assets/core_tuning/stat.tuning.ron`.

mod band;
mod body_part;
mod combat_tuning;
mod cone;
mod cone_groups;
mod economy;
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
    MuzzleForwardOffset, MuzzleHeight, RecoilClimb, SilhouetteTop, StabilityCurveCoord,
    StanceContribution,
};
pub use cone_groups::{
    AimMode, BraceMinHeight, ConcentrationCoeffs, ConeStabilityTuning, MuzzleHeights,
    SilhouetteTops, StabilityCurve, StabilityCurvePoint, StabilityCurves, StanceStability,
};
pub use economy::{LinkTu, MoveCost, MoveCosts, StanceChangeTu, TurnTu};
pub use firing_arc::FiringArc;
pub use matchup::MatchupMultipliers;
pub use melee::{FightVariance, MeleeKMargin, MeleeMultMax, MeleeMultMin, MeleeTuning};
pub use reaction::{
    ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin, ReactionProbability,
    ReactionScore, ReactionTuning, ReactionsUsed, clamp_probability, interrupt_probability,
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
