//! Aggregate combat balance resource.

use bevy::{prelude::Resource, reflect::TypePath};
use serde::Deserialize;

use crate::tuning::{
    band::ProjectileBandEdges,
    body_part::BodyPartWeights,
    cone_groups::ConeStabilityTuning,
    economy::{
        EnterEmplacementTu, ExitEmplacementTu, LinkTu, MoveCosts, OpenDoorTu, ShoveTu,
        StanceChangeTu, ThrowTu, TurnTu,
    },
    falls::PerStoreyDamage,
    firing_arc::FiringArc,
    matchup::MatchupMultipliers,
    melee::MeleeTuning,
    reaction::ReactionTuning,
    severity::SeverityScaling,
    visibility::{ExploredDim, ViewRange},
    wounds::{BleedRate, ExecuteTu, StabilizeTu, WoundCosts},
};

/// All combat tuning knobs loaded as one resource.
#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize, TypePath)]
pub struct CombatTuning {
    /// Projectile range bands.
    pub projectile_band_edges: ProjectileBandEdges,
    /// Severity curve scaling.
    pub severity_scaling: SeverityScaling,
    /// Wound pool costs.
    pub wound_costs: WoundCosts,
    /// Bleed rate.
    pub bleed_rate: BleedRate,
    /// Stabilize TU.
    pub stabilize_tu: StabilizeTu,
    /// Execute TU.
    pub execute_tu: ExecuteTu,
    /// Stance change TU.
    pub stance_change_tu: StanceChangeTu,
    /// Facing turn TU.
    pub turn_tu: TurnTu,
    /// Shove TU.
    pub shove_tu: ShoveTu,
    /// Open door TU.
    pub open_door_tu: OpenDoorTu,
    /// Enter emplacement TU.
    pub enter_emplacement_tu: EnterEmplacementTu,
    /// Exit emplacement TU.
    pub exit_emplacement_tu: ExitEmplacementTu,
    /// Throw TU.
    pub throw_tu: ThrowTu,
    /// Per-terrain move costs.
    pub move_costs: MoveCosts,
    /// Vertical link TU.
    pub link_tu: LinkTu,
    /// Body-part hit weights.
    pub body_part_weights: BodyPartWeights,
    /// Cone / stability tuning.
    pub cone_stability: ConeStabilityTuning,
    /// Armor matchup multipliers.
    pub matchup_multipliers: MatchupMultipliers,
    /// Firing arc degrees.
    pub firing_arc: FiringArc,
    /// View range cells.
    pub view_range: ViewRange,
    /// Explored-tile dim factor (optional / legacy).
    pub explored_dim: ExploredDim,
    /// Reaction fire tuning.
    pub reaction: ReactionTuning,
    /// Melee tuning.
    pub melee: MeleeTuning,
    /// Fall damage per storey.
    pub per_storey_damage: PerStoreyDamage,
}
