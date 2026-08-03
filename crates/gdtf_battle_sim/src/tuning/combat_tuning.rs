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

#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize, TypePath)]
pub struct CombatTuning {
        pub projectile_band_edges: ProjectileBandEdges,
        pub severity_scaling:      SeverityScaling,
            pub wound_costs:           WoundCosts,
            pub bleed_rate:            BleedRate,
                pub stabilize_tu:          StabilizeTu,
                pub execute_tu:            ExecuteTu,
                pub stance_change_tu:      StanceChangeTu,
                pub turn_tu:               TurnTu,
                                pub shove_tu:              ShoveTu,
                            pub open_door_tu:          OpenDoorTu,
                        pub enter_emplacement_tu:  EnterEmplacementTu,
                        pub exit_emplacement_tu:   ExitEmplacementTu,
                                pub throw_tu:              ThrowTu,
                                pub move_costs:            MoveCosts,
                                pub link_tu:               LinkTu,
        pub body_part_weights:     BodyPartWeights,
            pub cone_stability:        ConeStabilityTuning,
            pub matchup_multipliers:   MatchupMultipliers,
                        pub firing_arc:            FiringArc,
                        pub view_range:            ViewRange,
        /// terrain. **DEPRECATED / UNUSED by the renderer as of GTW-348**: EXPLORED now renders
                        pub explored_dim:          ExploredDim,
                                            pub reaction:              ReactionTuning,
                                            pub melee:                 MeleeTuning,
                                    pub per_storey_damage:     PerStoreyDamage,
}
