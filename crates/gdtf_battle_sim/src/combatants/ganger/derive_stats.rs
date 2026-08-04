//! Derive combat stats from authored attributes and tuning weights.

use bevy::prelude::Deref;

use crate::{
    ganger::{
        Bottle, Fight, Hp, HpMax, Morale, Reactions, Shooting, Tu, TuMax, Wounds, WoundsMax,
        attributes::GangerAttributes,
    },
    tuning::GangerStatTuning,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(crate) struct StatMagnitude(f32);

impl StatMagnitude {
    #[must_use]
    pub(crate) const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PoolValue(u8);

impl PoolValue {
    #[must_use]
    pub(crate) const fn new(value: u8) -> Self {
        Self(value)
    }
}

/// All stats computed from attributes (never authored directly).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedStats {
    /// Shooting skill.
    pub shooting:   Shooting,
    /// Fight skill.
    pub fight:      Fight,
    /// Reactions skill.
    pub reactions:  Reactions,
    /// Morale.
    pub morale:     Morale,
    /// Starting TU.
    pub tu:         Tu,
    /// Max TU.
    pub tu_max:     TuMax,
    /// Starting HP.
    pub hp:         Hp,
    /// Max HP.
    pub hp_max:     HpMax,
    /// Starting wounds.
    pub wounds:     Wounds,
    /// Max wounds.
    pub wounds_max: WoundsMax,
    /// Bottle threshold.
    pub bottle:     Bottle,
}

const fn round_to_u16(value: StatMagnitude) -> Hp {
    let rounded = value.0.round();
    let clamped = rounded.clamp(0.0, u16::MAX as f32) as u16;
    Hp::new(clamped)
}

#[must_use]
pub(crate) fn weighted_sum(terms: &[(f32, f32)]) -> StatMagnitude {
    StatMagnitude::new(terms.iter().fold(0.0, |acc, &(weight, attribute)| {
        weight.mul_add(attribute, acc)
    }))
}

const fn round_to_u8(value: StatMagnitude) -> PoolValue {
    let rounded = value.0.round();
    let clamped = rounded.clamp(0.0, u8::MAX as f32) as u8;
    PoolValue::new(clamped)
}

/// Compute derived stats from attributes and tuning weights.
#[must_use]
pub fn derive_stats(attributes: &GangerAttributes, tuning: &GangerStatTuning) -> DerivedStats {
    let speed = *attributes.speed;
    let aim = *attributes.aim;
    let strength = *attributes.strength;
    let toughness = *attributes.toughness;
    let reflexes = *attributes.reflexes;
    let cool = *attributes.cool;
    let grit = *attributes.grit;

    let shooting = weighted_sum(&[
        (*tuning.shooting.aim, aim),
        (*tuning.shooting.reflexes, reflexes),
        (*tuning.shooting.cool, cool),
    ]);

    let fight = weighted_sum(&[
        (*tuning.fight.speed, speed),
        (*tuning.fight.strength, strength),
        (*tuning.fight.grit, grit),
        (*tuning.fight.cool, cool),
    ]);

    let reactions = weighted_sum(&[
        (*tuning.reactions.speed, speed),
        (*tuning.reactions.reflexes, reflexes),
        (*tuning.reactions.cool, cool),
    ]);

    let morale = weighted_sum(&[(*tuning.morale.grit, grit), (*tuning.morale.cool, cool)]);

    let tu_f = (*tuning.tu_per_speed).mul_add(speed, *tuning.tu_base);
    let tu = round_to_u8(StatMagnitude::new(tu_f));

    let hp = round_to_u16(weighted_sum(&[
        (*tuning.hp.grit, grit),
        (*tuning.hp.toughness, toughness),
        (*tuning.hp.cool, cool),
    ]));

    let wounds = round_to_u8(StatMagnitude::new(f32::from(*hp) / *tuning.wounds_per_hp));

    let bottle = round_to_u8(StatMagnitude::new(*morale / *tuning.bottle_per_morale));

    DerivedStats {
        shooting: Shooting::new(*shooting),
        fight: Fight::new(*fight),
        reactions: Reactions::new(*reactions),
        morale: Morale::new(*morale),
        tu: Tu::new(*tu),
        tu_max: TuMax::new(*tu),
        hp,
        hp_max: HpMax::new(*hp),
        wounds: Wounds::new(*wounds),
        wounds_max: WoundsMax::new(*wounds),
        bottle: Bottle::new(*bottle),
    }
}
