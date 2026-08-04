//! Resolve one round inside a volley: cone, march, primary hit, splash.

use bevy::prelude::Entity;

use super::{
    super::query::{BattleGrids, StruckBodies},
    fold::resolve_primary_report,
    snapshot::ShooterSnapshot,
    splash::{PrimaryImpact, apply_aoe_splash},
};
use crate::{
    aim::{cone_for, stability_for},
    cover::CoverLedger,
    effects::attachments::WeaponBraceBonus,
    ganger::{LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    resolve_and_apply::{HitReport, WoundRoll},
    resolve_coarse::{ShotInputs, resolve_coarse},
    rng::ShotRng,
    sample_cone::concentration_p,
    stability::{StabilityTerms, terrain_brace::terrain_braces},
    weapon::FireModeSpec,
};

/// Target position, stance, and cover band for aim.
#[derive(Debug, Clone, Copy)]
pub(in crate::shot_pipeline::fire) struct TargetGeometry {
    position:   Position,
    stance:     Stance,
    cover_band: Option<crate::cover::HeightBand>,
}

impl TargetGeometry {
    /// Build geometry from the ordered target cell and current cover/occupancy.
    pub(in crate::shot_pipeline::fire) fn compose(
        target_cell: Cell,
        target_level: Level,
        cover: &CoverLedger,
        occupancy: &OccupancyGrid,
    ) -> Self {
        let at = CellLevel::new(target_cell, target_level);
        let aim_band = cover
            .peek(&at)
            .map(|entry| entry.height_band)
            .or_else(|| occupancy.occupant_band(&at));
        Self {
            position:   Position::new(at),
            stance:     Stance::new(StanceKind::Standing),
            cover_band: aim_band,
        }
    }
}

/// Inputs fixed for every round in the volley.
#[derive(Clone, Copy)]
pub(in crate::shot_pipeline::fire) struct RoundSetup<'a> {
    pub(in crate::shot_pipeline::fire) snapshot: &'a ShooterSnapshot,
    pub(in crate::shot_pipeline::fire) geometry: TargetGeometry,
    pub(in crate::shot_pipeline::fire) mode:     &'a FireModeSpec,
}

/// Resolve one round: build cone, march, fold primary hit, apply AOE splash.
pub(in crate::shot_pipeline::fire) fn resolve_round(
    setup: RoundSetup,
    prior_shots: crate::cone::PriorShots,
    grids: &mut BattleGrids,
    bodies: &mut StruckBodies,
    shot_rng: &mut ShotRng,
    roll: &mut WoundRoll<'_>,
) -> (
    HitReport,
    Vec<HitReport>,
    crate::resolve_coarse::ShotOutcome,
) {
    let tuning = roll.tuning;
    let snapshot = setup.snapshot;
    let geometry = setup.geometry;
    let shooter_view = snapshot.shooter_view();
    let terrain_braced = terrain_braces(
        snapshot.position,
        *snapshot.stance,
        grids.brace_cells,
        grids.surface,
    );
    let stability_terms = StabilityTerms {
        stable: snapshot.stable,
        terrain_braced,
        brace_bonus: snapshot.brace_bonus.unwrap_or_else(WeaponBraceBonus::none),
        emplacement: snapshot.emplacement_stability(tuning),
    };
    let cone = cone_for(
        &shooter_view,
        snapshot.weapon_stats(),
        setup.mode,
        prior_shots,
        grids.cover,
        stability_terms,
        tuning,
    );
    let (_cone_mult, recoil_growth) =
        stability_for(&shooter_view, stability_terms, grids.cover, tuning);
    let p = concentration_p(
        snapshot.shooting,
        snapshot.accuracy,
        tuning.cone_stability.concentration,
    );

    let shot = ShotInputs {
        shooter_position: snapshot.position,
        shooter_facing: snapshot.facing,
        shooter_stance: snapshot.stance,
        target_position: geometry.position,
        target_stance: geometry.stance,
        cover_band: geometry.cover_band,
        cone,
        p,
        prior_shots,
        recoil_climb: tuning.cone_stability.recoil_climb,
        recoil_growth,
    };

    let targets = &bodies.targets;
    let is_dead = |e: Entity| {
        targets
            .get(e)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };
    let outcome = resolve_coarse(
        &shot,
        grids.occupancy,
        grids.surface,
        grids.cover,
        tuning,
        shot_rng,
        is_dead,
    );

    let report = resolve_primary_report(&outcome, snapshot, grids, bodies, roll);

    let splash = apply_aoe_splash(
        PrimaryImpact {
            outcome:  &outcome,
            hit_type: setup.mode.hit_type,
            report:   &report,
        },
        snapshot,
        grids,
        bodies,
        shot_rng,
        roll,
    );

    (report, splash, outcome)
}
