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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
                Ganger(Entity),
            Cover(CoverEntry),
            Slab(CellLevel),
            Ground(CellLevel),
                Miss,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotOutcome {
            pub kind:       ShotKind,
        pub cell:       Cell,
            pub level:      Level,
            pub body_part:  Option<BodyPart>,
            pub band:       HeightBand,
            pub muzzle:     SimPos,
            pub trajectory: ShotDir,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotInputs {
        pub shooter_position: Position,
        pub shooter_facing:   Facing,
        pub shooter_stance:   Stance,
        pub target_position:  Position,
        pub target_stance:    Stance,
                pub cover_band:       Option<HeightBand>,
                    pub cone:             ConeAngle,
            pub p:                ConcentrationP,
            pub prior_shots:      PriorShots,
        pub recoil_climb:     RecoilClimb,
            pub recoil_growth:    RecoilGrowth,
}

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
