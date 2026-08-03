//! Fire a multi-round volley from one shooter.

use bevy::prelude::Entity;

use super::{
    compose::{RoundSetup, ShooterReads, TargetGeometry, read_shooter, resolve_round},
    query::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery,
        WeaponQuery, WearsQuery, WieldsQuery,
    },
};
use crate::{
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{FireActor, can_fire, clamp_burst, mode_tu_cost},
    resolve_and_apply::HitReport,
    resolve_coarse::ShotOutcome,
    rng::{InjuryRng, SeverityRng, ShotRng},
    tu::spend_tu,
    tuning::CombatTuning,
};

/// Result of firing one volley: per-round reports, outcomes, and splash.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Volley {
    /// Hit reports for each primary round.
    pub reports: Vec<HitReport>,
    /// Coarse shot outcomes for each round.
    pub shots: Vec<ShotOutcome>,
    /// AOE splash reports nested per primary round.
    pub splash: Vec<Vec<HitReport>>,
}

impl Volley {
    /// Empty volley (failed preconditions).
    pub(crate) const fn empty() -> Self {
        Self {
            reports: Vec::new(),
            shots: Vec::new(),
            splash: Vec::new(),
        }
    }
}

/// Fire a volley: spend TU, clamp burst to magazine, resolve each round.
///
/// Returns an empty volley when the shooter cannot fire.
#[expect(
    clippy::too_many_arguments,
    reason = "wears/pieces and wields/weapons stay disjoint; injury tables and InjuryRng are separate streams"
)]
pub fn fire(
    shooter: Entity,
    order: FireOrder,
    shooters: &mut ShooterQuery,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    wields: &WieldsQuery,
    weapons: &mut WeaponQuery,
    melee: &MeleeQuery,
    mounted: &MountedQuery,
    mut grids: BattleGrids,
    tuning: &CombatTuning,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> Volley {
    let Some(ShooterReads {
        snapshot,
        weapon: weapon_entity,
        tu: shooter_tu,
        tu_max: shooter_tu_max,
        aiming: shooter_aiming,
        magazine: magazine_now,
        handedness: shooter_handedness,
        hands_available: shooter_hands,
    }) = read_shooter(shooter, shooters, wields, weapons, melee, mounted)
    else {
        return Volley::empty();
    };

    let Ok((_, _, shooter_life, ..)) = targets.get(shooter) else {
        return Volley::empty();
    };
    let shooter_life = *shooter_life;

    let actor = FireActor {
        life: &shooter_life,
        tu: &shooter_tu,
        tu_max: &shooter_tu_max,
        aiming: &shooter_aiming,
        magazine: &magazine_now,
        handedness: shooter_handedness,
        hands_available: shooter_hands,
    };
    if !*can_fire(
        &actor,
        order.mode,
        order.target_cell,
        order.target_level,
        tuning,
    ) {
        return Volley::empty();
    }

    let charge = mode_tu_cost(order.mode, &shooter_tu_max, &shooter_aiming, tuning);
    if let Ok((_, _, mut tu_mut)) = shooters.get_mut(shooter) {
        spend_tu(&mut tu_mut, charge);
    } else {
        return Volley::empty();
    }

    let rounds = *clamp_burst(order.mode.shots, &magazine_now);

    let geometry = TargetGeometry::compose(
        order.target_cell,
        order.target_level,
        grids.cover,
        grids.occupancy,
    );
    let setup = RoundSetup {
        snapshot: &snapshot,
        geometry,
        mode: order.mode,
    };

    let mut reports = Vec::with_capacity(usize::from(rounds));
    let mut shots = Vec::with_capacity(usize::from(rounds));
    let mut splash = Vec::with_capacity(usize::from(rounds));
    for i in 0..rounds {
        let (report, round_splash, outcome) = resolve_round(
            setup,
            crate::cone::PriorShots::new(i),
            &mut grids,
            targets,
            wears,
            pieces,
            tuning,
            shot_rng,
            severity_rng,
            tables,
            registry,
            injury_rng,
        );

        if let Ok((.., mut mag_mut, _dot)) = weapons.get_mut(weapon_entity) {
            mag_mut.spend_round();
        }

        reports.push(report);
        shots.push(outcome);
        splash.push(round_splash);
    }

    Volley {
        reports,
        shots,
        splash,
    }
}
