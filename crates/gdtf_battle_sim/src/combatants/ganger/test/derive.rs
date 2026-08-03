use bevy::{
    MinimalPlugins,
    prelude::{App, Update, World},
};

use crate::{
    ganger::derive_stats::weighted_sum,
    ganger::{
        Aim, Cool, GangerAttributes, Grit, Hp, HpMax, Luck, Reflexes, Shooting, Speed, Strength,
        Toughness, derive_stats, rederive_stats_on_tuning_change,
    },
    tuning::{
        BottlePerMorale, FightWeights, GangerStatTuning, HpWeights, MoraleWeights,
        ReactionsWeights, ShootingWeights, StatWeight, TuBase, TuPerSpeed, WoundsPerHp,
    },
};

fn distinct_tuning() -> GangerStatTuning {
    GangerStatTuning {
        shooting:          ShootingWeights {
            aim:      StatWeight::new(1.5),
            reflexes: StatWeight::new(0.25),
            cool:     StatWeight::new(0.5),
        },
        fight:             FightWeights {
            speed:    StatWeight::new(0.3),
            strength: StatWeight::new(1.2),
            grit:     StatWeight::new(0.4),
            cool:     StatWeight::new(0.6),
        },
        reactions:         ReactionsWeights {
            speed:    StatWeight::new(0.7),
            reflexes: StatWeight::new(1.1),
            cool:     StatWeight::new(0.2),
        },
        hp:                HpWeights {
            grit:      StatWeight::new(1.3),
            toughness: StatWeight::new(0.9),
            cool:      StatWeight::new(0.5),
        },
        morale:            MoraleWeights {
            grit: StatWeight::new(1.4),
            cool: StatWeight::new(0.8),
        },
        wounds_per_hp:     WoundsPerHp::new(8.0),
        bottle_per_morale: BottlePerMorale::new(9.0),
        tu_base:           TuBase::new(25.0),
        tu_per_speed:      TuPerSpeed::new(7.0),
    }
}

fn sample_attributes() -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(4.0),
        aim:       Aim::new(6.0),
        strength:  Strength::new(5.0),
        toughness: Toughness::new(11.0),
        reflexes:  Reflexes::new(3.0),
        cool:      Cool::new(7.0),
        grit:      Grit::new(20.0),
        luck:      Luck::new(2.0),
    }
}

fn round_pool_u16(value: f32) -> u16 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the test's sample magnitudes round well inside u16 range; mirrors the \
                  derivation's round-to-nearest pool rule"
    )]
    let rounded = value.round().clamp(0.0, f32::from(u16::MAX)) as u16;
    rounded
}

fn round_pool_u8(value: f32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the test's sample magnitudes round well inside u8 range; mirrors the \
                  derivation's round-to-nearest pool rule"
    )]
    let rounded = value.round().clamp(0.0, f32::from(u8::MAX)) as u8;
    rounded
}

#[test]
fn each_derived_stat_equals_its_weighted_formula() {
    let tuning = distinct_tuning();
    let a = sample_attributes();
    let derived = derive_stats(&a, &tuning);

    let expected_shooting = weighted_sum(&[
        (*tuning.shooting.aim, *a.aim),
        (*tuning.shooting.reflexes, *a.reflexes),
        (*tuning.shooting.cool, *a.cool),
    ]);
    assert_eq!(
        derived.shooting.to_bits(),
        expected_shooting.to_bits(),
        "Shooting == aim_w·Aim + reflexes_w·Reflexes + cool_w·Cool",
    );

    let expected_fight = weighted_sum(&[
        (*tuning.fight.speed, *a.speed),
        (*tuning.fight.strength, *a.strength),
        (*tuning.fight.grit, *a.grit),
        (*tuning.fight.cool, *a.cool),
    ]);
    assert_eq!(
        derived.fight.to_bits(),
        expected_fight.to_bits(),
        "Fight == its weighted formula",
    );

    let expected_reactions = weighted_sum(&[
        (*tuning.reactions.speed, *a.speed),
        (*tuning.reactions.reflexes, *a.reflexes),
        (*tuning.reactions.cool, *a.cool),
    ]);
    assert_eq!(
        derived.reactions.to_bits(),
        expected_reactions.to_bits(),
        "Reactions == its weighted formula",
    );

    let expected_morale = weighted_sum(&[
        (*tuning.morale.grit, *a.grit),
        (*tuning.morale.cool, *a.cool),
    ]);
    assert_eq!(
        derived.morale.to_bits(),
        expected_morale.to_bits(),
        "Morale == its weighted formula",
    );

    let expected_tu = round_pool_u8((*tuning.tu_per_speed).mul_add(*a.speed, *tuning.tu_base));
    assert_eq!(
        *derived.tu, expected_tu,
        "TU == round(tu_base + tu_per_speed·Speed)",
    );

    let expected_hp = round_pool_u16(*weighted_sum(&[
        (*tuning.hp.grit, *a.grit),
        (*tuning.hp.toughness, *a.toughness),
        (*tuning.hp.cool, *a.cool),
    ]));
    assert_eq!(
        *derived.hp, expected_hp,
        "HP == round(grit·Grit + toughness·Toughness + cool·Cool)",
    );

    let expected_wounds = round_pool_u8(f32::from(expected_hp) / *tuning.wounds_per_hp);
    assert_eq!(
        *derived.wounds, expected_wounds,
        "Wounds == round(HP / wounds_per_hp)",
    );

    let expected_bottle = round_pool_u8(*expected_morale / *tuning.bottle_per_morale);
    assert_eq!(
        *derived.bottle, expected_bottle,
        "Bottle == round(Morale / bottle_per_morale)",
    );

    assert_eq!(*derived.hp_max, *derived.hp, "HpMax == derived HP");
    assert_eq!(
        *derived.wounds_max, *derived.wounds,
        "WoundsMax == derived Wounds"
    );
    assert_eq!(*derived.tu_max, *derived.tu, "TuMax == derived TU");
}

#[test]
fn raising_aim_raises_derived_shooting() {
    let tuning = GangerStatTuning::default();
    let base = sample_attributes();
    let mut higher = base;
    higher.aim = Aim::new(*base.aim + 5.0);

    let low = derive_stats(&base, &tuning);
    let high = derive_stats(&higher, &tuning);
    assert!(
        *high.shooting > *low.shooting,
        "a higher Aim must raise the derived Shooting (the term wires Aim)",
    );
}

fn rederive_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, rederive_stats_on_tuning_change);
    app
}

fn spawn_derived_ganger(
    world: &mut World,
    a: &GangerAttributes,
    tuning: &GangerStatTuning,
) -> bevy::prelude::Entity {
    let d = derive_stats(a, tuning);
    world
        .spawn((
            a.speed,
            a.aim,
            a.strength,
            a.toughness,
            a.reflexes,
            a.cool,
            a.grit,
            a.luck,
        ))
        .insert((d.shooting, d.fight, d.reactions, d.morale))
        .insert((
            d.tu,
            d.tu_max,
            d.hp,
            d.hp_max,
            d.wounds,
            d.wounds_max,
            d.bottle,
        ))
        .id()
}

#[test]
fn tuning_edit_rederives_spawned_ganger_stats() {
    let mut app = rederive_app();
    let attrs = sample_attributes();
    let baseline = GangerStatTuning::default();
    app.world_mut().insert_resource(baseline.clone());
    let entity = spawn_derived_ganger(app.world_mut(), &attrs, &baseline);
    app.update();
    let baseline_shooting = app.world().get::<Shooting>(entity).map(|s| **s);

    let mut edited = baseline.clone();
    edited.shooting.aim = StatWeight::new((*baseline.shooting.aim).mul_add(2.0, 1.0));
    let expected = derive_stats(&attrs, &edited).shooting;
    app.world_mut().insert_resource(edited);
    app.update();

    let after = app.world().get::<Shooting>(entity).map(|s| **s);
    assert_eq!(
        after,
        Some(*expected),
        "a tuning edit must re-derive the spawned ganger's Shooting to the new formula",
    );
    assert_ne!(
        after, baseline_shooting,
        "the re-derived Shooting must differ from the baseline (the edit took effect)",
    );
}

#[test]
fn rederive_clamps_damaged_pool_does_not_reset_to_full() {
    let mut app = rederive_app();
    let attrs = sample_attributes();
    let baseline = GangerStatTuning::default();
    app.world_mut().insert_resource(baseline.clone());
    let entity = spawn_derived_ganger(app.world_mut(), &attrs, &baseline);
    app.update();

    let derived_max = *derive_stats(&attrs, &baseline).hp_max;
    assert!(
        derived_max > 1,
        "precondition: the derived HP max exceeds 1"
    );
    if let Some(mut hp) = app.world_mut().get_mut::<Hp>(entity) {
        *hp = Hp::new(1);
    }

    let mut edited = baseline.clone();
    edited.hp.grit = StatWeight::new(*baseline.hp.grit + 1.0);
    let new_max = *derive_stats(&attrs, &edited).hp_max;
    assert!(
        new_max > 1,
        "precondition: the new derived HP max still exceeds 1"
    );
    app.world_mut().insert_resource(edited);
    app.update();

    let hp_after = app.world().get::<Hp>(entity).map(|h| **h);
    let hp_max_after = app.world().get::<HpMax>(entity).map(|h| **h);
    assert_eq!(
        hp_after,
        Some(1),
        "the DAMAGED current Hp (1) must be CLAMPED, not reset to full, on a re-derive",
    );
    assert_eq!(
        hp_max_after,
        Some(new_max),
        "the HpMax must take the NEW derived max (the ceiling re-derives)",
    );
}

#[test]
fn rederive_clamps_current_pool_down_to_a_lowered_max() {
    let mut app = rederive_app();
    let attrs = sample_attributes();
    let baseline = GangerStatTuning::default();
    app.world_mut().insert_resource(baseline.clone());
    let entity = spawn_derived_ganger(app.world_mut(), &attrs, &baseline);
    app.update();

    let mut edited = baseline;
    edited.hp = HpWeights {
        grit:      StatWeight::new(0.0),
        toughness: StatWeight::new(0.0),
        cool:      StatWeight::new(0.0),
    };
    let new_max = *derive_stats(&attrs, &edited).hp_max;
    app.world_mut().insert_resource(edited);
    app.update();

    let hp_after = app.world().get::<Hp>(entity).map(|h| **h);
    assert_eq!(
        hp_after,
        Some(new_max),
        "a lowered max must clamp the current Hp DOWN to it (never above the ceiling)",
    );
}
