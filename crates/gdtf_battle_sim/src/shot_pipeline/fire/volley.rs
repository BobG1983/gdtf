//! Fire a multi-round volley from one shooter.

use super::{
    compose::{RoundSetup, ShooterReads, TargetGeometry, read_shooter, resolve_round},
    query::{BattleGrids, FireOrder, ShooterQuery, StruckBodies, WieldedWeapons},
};
use crate::{
    magazine::{FireActor, can_fire, clamp_burst, mode_tu_cost},
    resolve_and_apply::{HitReport, WoundRoll},
    resolve_coarse::ShotOutcome,
    rng::ShotRng,
    tu::spend_tu,
};

/// Result of firing one volley: per-round reports, outcomes, and splash.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Volley {
    /// Hit reports for each primary round.
    pub reports: Vec<HitReport>,
    /// Coarse shot outcomes for each round.
    pub shots:   Vec<ShotOutcome>,
    /// AOE splash reports nested per primary round.
    pub splash:  Vec<Vec<HitReport>>,
}

impl Volley {
    /// Empty volley (failed preconditions).
    pub(crate) const fn empty() -> Self {
        Self {
            reports: Vec::new(),
            shots:   Vec::new(),
            splash:  Vec::new(),
        }
    }
}

/// Fire a volley: spend TU, clamp burst to magazine, resolve each round.
/// Returns an empty volley when the shooter cannot fire.
pub fn fire(
    order: FireOrder,
    shooters: &mut ShooterQuery,
    arms: &mut WieldedWeapons,
    bodies: &mut StruckBodies,
    mut grids: BattleGrids,
    shot_rng: &mut ShotRng,
    roll: &mut WoundRoll<'_>,
) -> Volley {
    let shooter = order.shooter;
    let tuning = roll.tuning;
    let Some(ShooterReads {
        snapshot,
        weapon: weapon_entity,
        tu: shooter_tu,
        tu_max: shooter_tu_max,
        aiming: shooter_aiming,
        magazine: magazine_now,
        handedness: shooter_handedness,
        hands_available: shooter_hands,
    }) = read_shooter(
        shooter,
        shooters,
        &arms.wields,
        &arms.weapons,
        &arms.melee,
        &arms.mounted,
    )
    else {
        return Volley::empty();
    };

    let Ok((_, _, shooter_life, ..)) = bodies.targets.get(shooter) else {
        return Volley::empty();
    };
    let shooter_life = *shooter_life;

    let actor = FireActor {
        life:            &shooter_life,
        tu:              &shooter_tu,
        tu_max:          &shooter_tu_max,
        aiming:          &shooter_aiming,
        magazine:        &magazine_now,
        handedness:      shooter_handedness,
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
    let Ok((_, _, mut tu_mut)) = shooters.get_mut(shooter) else {
        return Volley::empty();
    };
    if spend_tu(&mut tu_mut, charge).is_err() {
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
            bodies,
            shot_rng,
            roll,
        );

        if let Ok((.., mut mag_mut, _dot)) = arms.weapons.get_mut(weapon_entity) {
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
