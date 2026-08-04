//! Project injury deltas onto attributes and derived stats without mutating bases.

use bevy::prelude::Deref;

use crate::{
    ganger::{
        Aim, Bottle, Cool, DerivedStats, Fight, GangerAttributes, Grit, Hp, HpMax, Luck, Morale,
        Reactions, Reflexes, Shooting, Speed, Strength, Toughness, Tu, TuMax, Wounds, WoundsMax,
        derive_stats,
        derive_stats::{PoolValue, StatMagnitude},
    },
    injuries::{InflictedInjuries, StatDeltaSum, StatTarget},
    tuning::GangerStatTuning,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
struct AttributeValue(f32);

impl AttributeValue {
    #[must_use]
    const fn new(value: f32) -> Self {
        Self(value)
    }
}

fn effective_attr_value(base: AttributeValue, sum: StatDeltaSum) -> AttributeValue {
    AttributeValue::new(*base + f32::from(*sum))
}

/// Toughness after injury deltas.
#[must_use]
pub fn effective_toughness(base: Toughness, ledger: &InflictedInjuries) -> Toughness {
    Toughness::new(*effective_attr_value(
        AttributeValue::new(*base),
        ledger.delta_for(StatTarget::Toughness),
    ))
}

/// Luck after injury deltas.
#[must_use]
pub fn effective_luck(base: Luck, ledger: &InflictedInjuries) -> Luck {
    Luck::new(*effective_attr_value(
        AttributeValue::new(*base),
        ledger.delta_for(StatTarget::Luck),
    ))
}

fn effective_attributes(base: &GangerAttributes, ledger: &InflictedInjuries) -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(*effective_attr_value(
            AttributeValue::new(*base.speed),
            ledger.delta_for(StatTarget::Speed),
        )),
        aim:       Aim::new(*effective_attr_value(
            AttributeValue::new(*base.aim),
            ledger.delta_for(StatTarget::Aim),
        )),
        strength:  Strength::new(*effective_attr_value(
            AttributeValue::new(*base.strength),
            ledger.delta_for(StatTarget::Strength),
        )),
        toughness: effective_toughness(base.toughness, ledger),
        reflexes:  Reflexes::new(*effective_attr_value(
            AttributeValue::new(*base.reflexes),
            ledger.delta_for(StatTarget::Reflexes),
        )),
        cool:      Cool::new(*effective_attr_value(
            AttributeValue::new(*base.cool),
            ledger.delta_for(StatTarget::Cool),
        )),
        grit:      Grit::new(*effective_attr_value(
            AttributeValue::new(*base.grit),
            ledger.delta_for(StatTarget::Grit),
        )),
        luck:      effective_luck(base.luck, ledger),
    }
}

fn skill_with_delta(derived: StatMagnitude, sum: StatDeltaSum) -> StatMagnitude {
    StatMagnitude::new(*derived + f32::from(*sum))
}

fn pool_max_u8_with_delta(derived_max: PoolValue, sum: StatDeltaSum) -> PoolValue {
    let docked = i32::from(*derived_max) + i32::from(*sum);
    let floor = i32::from(*derived_max).min(1);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to floor..=u8::MAX above (floor is 0 or 1), so the cast cannot \
                  wrap or go negative"
    )]
    let floored = docked.clamp(floor, i32::from(u8::MAX)) as u8;
    PoolValue::new(floored)
}

fn pool_max_u16_with_delta(derived_max: Hp, sum: StatDeltaSum) -> Hp {
    let docked = i32::from(*derived_max) + i32::from(*sum);
    let floor = i32::from(*derived_max).min(1);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to floor..=u16::MAX above (floor is 0 or 1), so the cast cannot \
                  wrap or go negative"
    )]
    let floored = docked.clamp(floor, i32::from(u16::MAX)) as u16;
    Hp::new(floored)
}

/// Derive skills and pools from attributes, then apply injury deltas.
#[must_use]
pub fn derive_stats_with_injuries(
    base: &GangerAttributes,
    tuning: &GangerStatTuning,
    ledger: &InflictedInjuries,
) -> DerivedStats {
    let effective = effective_attributes(base, ledger);
    let derived = derive_stats(&effective, tuning);

    let shooting = Shooting::new(*skill_with_delta(
        StatMagnitude::new(*derived.shooting),
        ledger.delta_for(StatTarget::Shooting),
    ));
    let fight = Fight::new(*skill_with_delta(
        StatMagnitude::new(*derived.fight),
        ledger.delta_for(StatTarget::Fight),
    ));
    let reactions = Reactions::new(*skill_with_delta(
        StatMagnitude::new(*derived.reactions),
        ledger.delta_for(StatTarget::Reactions),
    ));
    let morale = Morale::new(*skill_with_delta(
        StatMagnitude::new(*derived.morale),
        ledger.delta_for(StatTarget::Morale),
    ));

    let tu_max = pool_max_u8_with_delta(
        PoolValue::new(*derived.tu_max),
        ledger.delta_for(StatTarget::Tu),
    );
    let hp_max =
        pool_max_u16_with_delta(Hp::new(*derived.hp_max), ledger.delta_for(StatTarget::Hp));
    let wounds_max = pool_max_u8_with_delta(
        PoolValue::new(*derived.wounds_max),
        ledger.delta_for(StatTarget::Wounds),
    );
    let bottle = pool_max_u8_with_delta(
        PoolValue::new(*derived.bottle),
        ledger.delta_for(StatTarget::Bottle),
    );

    DerivedStats {
        shooting,
        fight,
        reactions,
        morale,
        tu: Tu::new(*tu_max),
        tu_max: TuMax::new(*tu_max),
        hp: hp_max,
        hp_max: HpMax::new(*hp_max),
        wounds: Wounds::new(*wounds_max),
        wounds_max: WoundsMax::new(*wounds_max),
        bottle: Bottle::new(*bottle),
    }
}
