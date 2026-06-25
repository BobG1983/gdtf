//! The top-level [`CombatTuning`] resource — every balance coefficient the sim
//! marches with, composed from the per-domain sub-tuning structs.

use bevy::{prelude::Resource, reflect::TypePath};
use serde::Deserialize;

use crate::tuning::{
    band::ProjectileBandEdges,
    body_part::BodyPartWeights,
    cone_groups::ConeStabilityTuning,
    economy::{LinkTu, MoveCosts, StanceChangeTu, TurnTu},
    firing_arc::FiringArc,
    matchup::MatchupMultipliers,
    severity::SeverityScaling,
    slab::SlabDefaults,
    visibility::{ExploredDim, ViewRange},
    wounds::{BleedRate, ExecuteTu, StabilizeTu, WoundCosts},
};

/// The combat tuning resource — every balance coefficient the sim marches with.
///
/// A Bevy [`Resource`] deserializable from a `.ron` file (the tuning store is a
/// serde-loaded resource, resolution.md §"Coefficients live in the
/// combat-tuning data"). Holds the clearance band edges, the §6 severity
/// scaling, the body-part weights, and the E2.1 §1 cone/stability/recoil/aim
/// extension ([`ConeStabilityTuning`]); more sub-fields land as the systems do.
/// Defaults carry the doc values, but they are **tunable** — a data file
/// overrides any of them.
///
/// Derives [`TypePath`] (render-free reflection metadata, no rendering) because
/// the GTW-206 (E10.4) `Load` scene loads it through the `RonAsset<T>` loader,
/// whose payload bound requires `T: TypePath` — the same bound the theme spec and
/// authored situation satisfy.
#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize, TypePath)]
pub struct CombatTuning {
    /// Projectile clearance band edges (the LOW/MID/HIGH thresholds).
    pub projectile_band_edges: ProjectileBandEdges,
    /// The resolution.md §6 wound-severity scaling scalars.
    pub severity_scaling:      SeverityScaling,
    /// The per-tier Wounds-budget costs (E3.6) — Minor / Major / Critical spend
    /// (None = 0 and Fatal = empty are structural, not authored here).
    pub wound_costs:           WoundCosts,
    /// The §9 bleed-out rate (E3.7) — the flat Wounds an un-stabilized Downed
    /// ganger loses each round `tick_bleed` runs.
    pub bleed_rate:            BleedRate,
    /// The §9 stabilize TU cost (E3.8) — the flat Time Units an adjacent ally
    /// spends to halt a Downed ganger's bleed clock (`stabilize_downed` READS this;
    /// the TU economy that debits it is E4).
    pub stabilize_tu:          StabilizeTu,
    /// The §9 execute TU cost (E3.8) — the flat Time Units an adjacent enemy spends
    /// to finish a Downed ganger outright (`execute_downed` READS this; the TU
    /// economy that debits it is E4).
    pub execute_tu:            ExecuteTu,
    /// The stance-change TU cost (E4.1) — the flat Time Units `set_stance` spends
    /// via [`crate::tu::spend_tu`] when a ganger's posture actually changes
    /// (combat.md L34 "kneel" costs TUs; resolution.md §"What's tunable").
    pub stance_change_tu:      StanceChangeTu,
    /// The turn TU cost (E4.1) — the flat Time Units `set_facing` spends via
    /// [`crate::tu::spend_tu`] when a ganger's facing actually changes (combat.md
    /// L34 "turn" costs TUs; magnitude is tunable, mirroring `stance_change_tu`).
    pub turn_tu:               TurnTu,
    /// The per-terrain move-cost table (movement) — the flat Time Units
    /// [`crate::move_acts::advance_walk`] spends via [`crate::tu::spend_tu`] to step onto
    /// a destination cell, keyed by that cell's
    /// [`TerrainKind`](crate::occupancy::TerrainKind) (the floor tile crossed determines
    /// the cost; combat.md L34 "step" costs TUs). Terrain-determined, NOT a
    /// flat per-cell constant; magnitudes are tunable starting points (flagged), mirroring
    /// `stance_change_tu` / `turn_tu`.
    pub move_costs:            MoveCosts,
    /// The per-link traversal TU cost (E7 · GTW-12a) — the flat Time Units a step spends
    /// when it is a vertical-link hop (stair / ladder) instead of a terrain step: a
    /// crossing prices `link_tu` *instead of* the destination terrain's
    /// [`MoveCost`](crate::tuning::MoveCost) (visibility.md §48). ONE flat cost for every
    /// link kind (no per-kind split). Sim-authored; **consumed later by GTW-351** (the
    /// vertical-link traversal verb) — this leaf only ADDS the tunable, no movement code
    /// reads it yet. Tunable starting point, mirroring the other economy leaves.
    pub link_tu:               LinkTu,
    /// The §4 body-part hit-location weights.
    pub body_part_weights:     BodyPartWeights,
    /// The §1 cone / stability / recoil / aim coefficients (E2.1) — the data
    /// substrate for the rest of E2.
    pub cone_stability:        ConeStabilityTuning,
    /// The 7-type matchup multipliers (E3.2) — the favorable / neutral / resisted
    /// punch-&-shred scalars.
    pub matchup_multipliers:   MatchupMultipliers,
    /// The firing arc (GTW-242) — the full angular width (degrees) of the facing cone a
    /// shooter may fire within before it must turn to face the target. A target outside
    /// `±firing_arc / 2` requires the shooter to turn-into-arc AND afford the shot, else
    /// the shot is rejected. GLOBAL (one arc for all weapons this slice); magnitude is
    /// tunable (default 120°), mirroring the other tuning leaves.
    pub firing_arc:            FiringArc,
    /// The squad-FOV view range (GTW-338) — one ganger's sight radius in Chebyshev
    /// cells, the 2D disc bounding per-ganger FOV before the LOS probe owns the height
    /// axis (visibility.md §"Tunables"). Default 14: the 60×60 city gets a real fog
    /// horizon, the 12×12 fixtures read fully lit. Sim-authored; consumed by the
    /// presenter fog writer (GTW-342). Tunable, mirroring the other tuning leaves.
    pub view_range:            ViewRange,
    /// The EXPLORED-memory dim factor (GTW-338) — historically the modulate on EXPLORED
    /// terrain. **DEPRECATED / UNUSED by the renderer as of GTW-348**: EXPLORED now renders
    /// full-brightness GREYSCALE (colour-loss as the memory cue, not brightness-loss), so the
    /// presenter fog writer no longer reads this factor (see [`ExploredDim`]). Retained as a
    /// valid, parseable leaf (GTW-348 is presenter-only — pruning it would ripple into the
    /// model + RON + the parse/default tests); a future ticket repurposes or retires it.
    /// Tunable, mirroring the other tuning leaves.
    pub explored_dim:          ExploredDim,
    /// The slab-defaults (GTW-365) — the uniform structural HP + armor every floor/roof
    /// slab lazily seeds to when first struck (`docs/combat/resolution.md` §3.1;
    /// user-ruled 2026-06-22). Slabs are uniform level structure (authored as a bare
    /// `(cell, level)` list with NO per-slab HP), so this combat-tuning leaf is the sole
    /// HP/armor source for a struck slab — **genuinely consumed** by the
    /// [`SlabLedger`](crate::slab::SlabLedger)'s lazy-seed (`SlabLedger::prototype_for`)
    /// on the live depletion path (C7: no dead leaf). Tunable, mirroring the other leaves.
    pub slab_defaults:         SlabDefaults,
}
