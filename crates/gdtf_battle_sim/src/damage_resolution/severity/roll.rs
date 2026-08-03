use bevy::prelude::Deref;

use super::kind::{PartSeverityMod, Severity, SeverityScore, bucket};
use crate::{
    ganger::{Luck, Toughness},
    resolve_hit::{DamageReal, PenetratingDamage},
    rng::SeverityRng,
    tuning::SeverityScaling,
    weapon::FatalBias,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(super) struct RollTerm(f32);

impl RollTerm {
        #[must_use]
    const fn new(term: f32) -> Self {
        Self(term)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeverityInputs {
        pub pen_damage:    PenetratingDamage,
        pub toughness:     Toughness,
        pub part_mod:      PartSeverityMod,
        pub fatal_bias:    FatalBias,
        pub luck_shooter:  Luck,
        pub luck_defender: Luck,
}

impl SeverityInputs {
                            #[must_use]
    pub const fn new(
        pen_damage: PenetratingDamage,
        toughness: Toughness,
        part_mod: PartSeverityMod,
        fatal_bias: FatalBias,
        luck_shooter: Luck,
        luck_defender: Luck,
    ) -> Self {
        Self {
            pen_damage,
            toughness,
            part_mod,
            fatal_bias,
            luck_shooter,
            luck_defender,
        }
    }
}

/// `#[expect]` is the crate's guarded-cast idiom (`metric::floor_to_i32` /
#[expect(
    clippy::cast_precision_loss,
    reason = "penetrating damage is a small non-negative count, far inside f32's exact-integer range"
)]
fn pen_to_f32(pen: PenetratingDamage) -> DamageReal {
    DamageReal::new(*pen as f32)
}

pub(super) fn roll_term(
    scaling: &SeverityScaling,
    luck_defender: Luck,
    rng: &mut SeverityRng,
) -> RollTerm {
    let lo = -*scaling.defender_luck_scale * *luck_defender;
    let hi = *scaling.random_spread;
    RollTerm::new(rng.random_range_or_midpoint(lo..hi))
}

pub(super) fn severity_score(
    inputs: &SeverityInputs,
    scaling: &SeverityScaling,
    rng: &mut SeverityRng,
) -> SeverityScore {
    let pen_term = *scaling.pen_damage_scale * *pen_to_f32(inputs.pen_damage);
    let toughness_term = *scaling.toughness_mitigation * *inputs.toughness;
    let shooter_term = *scaling.shooter_luck_scale * *inputs.luck_shooter;
    let roll = roll_term(scaling, inputs.luck_defender, rng);

    SeverityScore::new(
        pen_term - toughness_term + *inputs.part_mod + *inputs.fatal_bias + shooter_term + *roll,
    )
}

#[must_use]
pub fn roll_severity(
    inputs: &SeverityInputs,
    scaling: &SeverityScaling,
    rng: &mut SeverityRng,
) -> Severity {
    let score = severity_score(inputs, scaling, rng);
    bucket(score, scaling)
}
