//! Both systems re-derive a spawned ganger's computed stats from its eight authored
use bevy::{
    ecs::change_detection::DetectChanges,
    prelude::{Changed, Mut, Query, Res},
};

use crate::{
    ganger::{
        Aim, Bottle, Cool, Fight, GangerAttributes, Grit, Hp, HpMax, Luck, Morale, Reactions,
        Reflexes, Shooting, Speed, Strength, Toughness, Tu, TuMax, Wounds, WoundsMax,
        injury_projection::derive_stats_with_injuries,
    },
    injuries::InflictedInjuries,
    tuning::GangerStatTuning,
};

type AttributeRead<'w> = (
    &'w Speed,
    &'w Aim,
    &'w Strength,
    &'w Toughness,
    &'w Reflexes,
    &'w Cool,
    &'w Grit,
    &'w Luck,
);

type SkillWrite<'w> = (
    Mut<'w, Shooting>,
    Mut<'w, Fight>,
    Mut<'w, Reactions>,
    Mut<'w, Morale>,
);

type PoolWrite<'w> = (
    Mut<'w, Tu>,
    Mut<'w, TuMax>,
    Mut<'w, Hp>,
    Mut<'w, HpMax>,
    Mut<'w, Wounds>,
    Mut<'w, WoundsMax>,
    Mut<'w, Bottle>,
);

fn rederive_one(
    base: &GangerAttributes,
    tuning: &GangerStatTuning,
    ledger: &InflictedInjuries,
    skills: SkillWrite,
    pools: PoolWrite,
) {
    let derived = derive_stats_with_injuries(base, tuning, ledger);

    let (mut shooting, mut fight, mut reactions, mut morale) = skills;
    *shooting = derived.shooting;
    *fight = derived.fight;
    *reactions = derived.reactions;
    *morale = derived.morale;

    let (mut tu, mut tu_max, mut hp, mut hp_max, mut wounds, mut wounds_max, mut bottle) = pools;
    *tu_max = derived.tu_max;
    *tu = Tu::new((*tu).min(*derived.tu_max));
    *hp_max = derived.hp_max;
    *hp = Hp::new((*hp).min(*derived.hp_max));
    *wounds_max = derived.wounds_max;
    *wounds = Wounds::new((*wounds).min(*derived.wounds_max));
    *bottle = derived.bottle;
}

const fn attributes_of(read: &AttributeRead) -> GangerAttributes {
    let (speed, aim, strength, toughness, reflexes, cool, grit, luck) = *read;
    GangerAttributes {
        speed:     *speed,
        aim:       *aim,
        strength:  *strength,
        toughness: *toughness,
        reflexes:  *reflexes,
        cool:      *cool,
        grit:      *grit,
        luck:      *luck,
    }
}

pub fn rederive_stats_on_tuning_change(
    tuning: Option<Res<GangerStatTuning>>,
    mut gangers: Query<(
        AttributeRead,
        Option<&InflictedInjuries>,
        SkillWrite,
        PoolWrite,
    )>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    if !tuning.is_changed() {
        return;
    }
    let empty = InflictedInjuries::default();
    for (read, ledger, skills, pools) in &mut gangers {
        let base = attributes_of(&read);
        let ledger = ledger.unwrap_or(&empty);
        rederive_one(&base, &tuning, ledger, skills, pools);
    }
}

pub fn rederive_stats_on_injury_change(
    tuning: Option<Res<GangerStatTuning>>,
    mut gangers: Query<
        (AttributeRead, &InflictedInjuries, SkillWrite, PoolWrite),
        Changed<InflictedInjuries>,
    >,
) {
    let Some(tuning) = tuning else {
        return;
    };
    for (read, ledger, skills, pools) in &mut gangers {
        let base = attributes_of(&read);
        rederive_one(&base, &tuning, ledger, skills, pools);
    }
}
