use bevy::prelude::Deref;

use crate::{
    ganger::Fight,
    resolve_hit::{
        DamageMagnitude, DamageReal, HitResult, HpDamage, IntegrityWear, PenetratingDamage,
    },
    rng::FightRng,
    tuning::{FightVariance, MeleeTuning},
};


#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct FightMargin(f32);

impl FightMargin {
                        #[must_use]
    pub const fn new(margin: f32) -> Self {
        Self(margin)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct MeleeDamageMult(f32);

impl MeleeDamageMult {
                        #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FightOutcome {
        pub connect: Connected,
        pub margin:  FightMargin,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Connected(bool);

impl Connected {
        #[must_use]
    pub const fn new(connected: bool) -> Self {
        Self(connected)
    }
}


const DEGENERATE_MARGIN: f32 = 1.0e6;


#[must_use]
pub fn opposed_fight(
    attacker: Fight,
    defender: Fight,
    variance: FightVariance,
    rng: &mut FightRng,
) -> FightOutcome {
    let lo = 1.0 - *variance;
    let hi = 1.0 + *variance;
    let roll_atk: f32 = rng.random_range_or_midpoint(lo..hi);
    let roll_def: f32 = rng.random_range_or_midpoint(lo..hi);

    let atk = *attacker * roll_atk;
    let def = *defender * roll_def;

    if def <= 0.0 {
        return FightOutcome {
            connect: Connected::new(true),
            margin:  FightMargin::new(DEGENERATE_MARGIN),
        };
    }

    FightOutcome {
        connect: Connected::new(atk > def),
        margin:  FightMargin::new(atk / def - 1.0),
    }
}

#[must_use]
pub fn melee_damage_mult(margin: FightMargin, tuning: &MeleeTuning) -> MeleeDamageMult {
    let raw = (*tuning.k_margin).mul_add(*margin, *tuning.mult_min);
    MeleeDamageMult::new(raw.clamp(*tuning.mult_min, *tuning.mult_max))
}

/// clamp + localized `#[expect]` is the crate's guarded-cast idiom, so no
const fn round_to_i32(value: DamageReal) -> DamageMagnitude {
    let rounded = value.get().round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    DamageMagnitude::new(clamped)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "i32 damage → f32 for the melee multiply; resolved-damage magnitudes are far inside f32's exact-integer range"
)]
fn scale_damage(component: DamageMagnitude, mult: MeleeDamageMult) -> DamageMagnitude {
    round_to_i32(DamageReal::new(*component as f32 * *mult))
}

#[must_use]
pub fn apply_melee_multiplier(hit: HitResult, mult: MeleeDamageMult) -> HitResult {
    HitResult {
        penetrating: PenetratingDamage::new(*scale_damage(
            DamageMagnitude::new(*hit.penetrating),
            mult,
        )),
        hp_damage:   HpDamage::new(*scale_damage(DamageMagnitude::new(*hit.hp_damage), mult)),
        wear:        IntegrityWear::new(*scale_damage(DamageMagnitude::new(*hit.wear), mult)),
    }
}
