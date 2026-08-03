//! Coarse shot: aim, sample cone, march, classify impact.

use bevy::prelude::Entity;

use crate::{
    armor::BodyPart,
    central_axis::{climb_aim_dir, muzzle_position, target_aim_point},
    cone::{ConeAngle, PriorShots},
    cover::{CoverEntry, CoverLedger, HeightBand},
    ganger::{Facing, Position, Stance},
    hit_location::roll_body_part,
    march::{MarchDir, MarchKind, MarchResult, march_vector},
    metric::{Cell, CellLevel, Level, SimPos},
    occupancy::OccupancyGrid,
    rng::ShotRng,
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    stability::RecoilGrowth,
    surface::SurfaceGrid,
    tuning::{CombatTuning, RecoilClimb},
};

/// What a coarse march hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    /// Living combatant.
    Ganger(Entity),
    /// Cover entry.
    Cover(CoverEntry),
    /// Floor slab.
    Slab(CellLevel),
    /// Open ground accrual.
    Ground(CellLevel),
    /// No solid impact.
    Miss,
}

/// Result of one coarse shot resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotOutcome {
    /// Impact kind.
    pub kind: ShotKind,
    /// Impact cell.
    pub cell: Cell,
    /// Impact level.
    pub level: Level,
    /// Rolled body part when hitting a ganger.
    pub body_part: Option<BodyPart>,
    /// Height band at impact.
    pub band: HeightBand,
    /// Muzzle position.
    pub muzzle: SimPos,
    /// Sampled trajectory.
    pub trajectory: ShotDir,
}

/// Inputs needed to resolve one coarse shot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotInputs {
    /// Shooter position.
    pub shooter_position: Position,
    /// Shooter facing.
    pub shooter_facing: Facing,
    /// Shooter stance.
    pub shooter_stance: Stance,
    /// Target position.
    pub target_position: Position,
    /// Target stance.
    pub target_stance: Stance,
    /// Optional cover band at target.
    pub cover_band: Option<HeightBand>,
    /// Cone angle.
    pub cone: ConeAngle,
    /// Concentration probability.
    pub p: ConcentrationP,
    /// Shots already fired in this burst.
    pub prior_shots: PriorShots,
    /// Recoil climb tuning.
    pub recoil_climb: RecoilClimb,
    /// Recoil growth state.
    pub recoil_growth: RecoilGrowth,
}

/// Aim, sample the cone, march the ray, classify the impact.
#[must_use]
pub fn resolve_coarse(
    shot: &ShotInputs,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    rng: &mut ShotRng,
    is_dead: impl Fn(Entity) -> bool,
) -> ShotOutcome {
    let muzzle = muzzle_position(
        shot.shooter_position,
        shot.shooter_facing,
        shot.shooter_stance,
        tuning,
    );

    let aim_point = target_aim_point(
        shot.target_position,
        shot.target_stance,
        shot.cover_band,
        tuning,
    );
    let aim_dir = climb_aim_dir(
        muzzle,
        aim_point,
        shot.prior_shots,
        shot.recoil_climb,
        shot.recoil_growth,
    );

    let trajectory = sample_cone_vector(aim_dir, shot.cone, shot.p, rng.rng());

    let shooter_cell = *shot.shooter_position;
    let march = march_vector(
        muzzle,
        MarchDir::new(trajectory.vec()),
        occupancy,
        surface,
        cover,
        tuning,
        shooter_cell,
        is_dead,
    );

    outcome_from_march(march, muzzle, trajectory, tuning, rng)
}

fn outcome_from_march(
    march: MarchResult,
    muzzle: SimPos,
    trajectory: ShotDir,
    tuning: &CombatTuning,
    rng: &mut ShotRng,
) -> ShotOutcome {
    let (cell, level) = march.at.split();

    let (kind, body_part) = match march.kind {
        MarchKind::Ganger(entity) => {
            let part = roll_body_part(&tuning.body_part_weights, rng.rng());
            (ShotKind::Ganger(entity), Some(part))
        }
        MarchKind::Cover(entry) => (ShotKind::Cover(entry), None),
        MarchKind::Slab => (ShotKind::Slab(march.at), None),
        MarchKind::Ground => (ShotKind::Ground(march.at), None),
        MarchKind::Miss => (ShotKind::Miss, None),
    };

    ShotOutcome {
        kind,
        cell,
        level,
        body_part,
        band: march.band,
        muzzle,
        trajectory,
    }
}
