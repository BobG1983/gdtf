//! Compose stability and cone angle for a given shooter and weapon.

use crate::{
    aim::shooter::Shooter,
    cone::{ConeAngle, PriorShots, aim_cone_mult, cone_angle},
    cover::CoverLedger,
    faced_cell::faced_cell,
    metric::CellLevel,
    stability::{ConeMult, RecoilGrowth, StabilityTerms, SuppressionStability, stability},
    tuning::CombatTuning,
    weapon::{FireModeSpec, WeaponStats},
};

/// Stability multipliers for this shooter given stance, cover, and suppression.
#[must_use]
pub fn stability_for(
    shooter: &Shooter,
    terms: StabilityTerms,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> (ConeMult, RecoilGrowth) {
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let faced = cover.peek(&CellLevel::new(cell, level));
    let suppression = match shooter.suppressed {
        Some(_) => SuppressionStability::new(-*tuning.reaction.suppression_penalty),
        None => SuppressionStability::none(),
    };
    stability(
        terms,
        *shooter.stance,
        faced,
        suppression,
        &tuning.cone_stability,
    )
}

/// Full cone angle for this shot, including fire mode, prior shots, and aim state.
#[must_use]
pub fn cone_for(
    shooter: &Shooter,
    weapon: WeaponStats<'_>,
    mode: &FireModeSpec,
    prior_shots: PriorShots,
    cover: &CoverLedger,
    terms: StabilityTerms,
    tuning: &CombatTuning,
) -> ConeAngle {
    let (cone_mult, recoil_growth) = stability_for(shooter, terms, cover, tuning);
    let aim = aim_cone_mult(*shooter.aiming, &tuning.cone_stability);
    cone_angle(
        *weapon.base_spread,
        mode.cone_mult,
        prior_shots,
        *weapon.kickback,
        recoil_growth,
        cone_mult,
        aim,
    )
}
