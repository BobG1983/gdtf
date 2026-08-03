use bevy::{
    MinimalPlugins,
    prelude::{App, Entity, IntoScheduleConfigs, Update, World},
};

use crate::{
    acts::{InjuryInflicted, apply_injury},
    armor::BodyPart,
    ganger::{
        Aim, Cool, GangerAttributes, Grit, Luck, Reflexes, Shooting, Speed, Strength, Toughness,
        derive_stats_with_injuries, rederive_stats_on_injury_change,
    },
    injuries::{
        InflictedInjuries, InjuryEffect, InjuryName, InspectText, LogText, PopupText, RolledInjury,
        StatDelta, StatTarget,
    },
    severity::Severity,
    tuning::GangerStatTuning,
};

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

fn spawn_statted_ganger(world: &mut World, tuning: &GangerStatTuning) -> Entity {
    let a = sample_attributes();
    let ledger = InflictedInjuries::default();
    let d = derive_stats_with_injuries(&a, tuning, &ledger);
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
        .insert(ledger)
        .id()
}

fn shooting_penalty_message(target: Entity, penalty: i8) -> InjuryInflicted {
    InjuryInflicted::from_rolled(
        target,
        RolledInjury::new(
            InjuryName::new("seam_probe".to_owned()),
            BodyPart::Torso,
            Severity::Minor,
            vec![InjuryEffect::Modify {
                stat:   StatTarget::Shooting,
                amount: StatDelta::new(penalty),
            }],
            PopupText::new("SEAM".to_owned()),
            LogText::new("probes the seam".to_owned()),
            InspectText::new("seam probe".to_owned()),
        ),
    )
}

#[test]
fn injury_message_lands_stat_delta_the_same_tick() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<InjuryInflicted>()
        .add_systems(
            Update,
            (apply_injury, rederive_stats_on_injury_change).chain(),
        );
    let tuning = GangerStatTuning::default();
    app.insert_resource(tuning.clone());
    let entity = spawn_statted_ganger(app.world_mut(), &tuning);
    app.update();
    let Some(baseline) = app.world().get::<Shooting>(entity).map(|s| **s) else {
        unreachable!("the statted ganger must carry a derived Shooting");
    };

    let penalty: i8 = -3;
    app.world_mut()
        .write_message(shooting_penalty_message(entity, penalty));
    app.update();

    let gained = app
        .world()
        .get::<InflictedInjuries>(entity)
        .map_or(0, |l| l.gained().len());
    assert_eq!(gained, 1, "apply_injury drained the message this tick");
    let after = app.world().get::<Shooting>(entity).map(|s| **s);
    assert_eq!(
        after,
        Some(baseline + f32::from(penalty)),
        "the Modify(Shooting) delta must land on the derived stat within the SAME \
         update as the message drain (a deferred gain would leave the baseline)",
    );
}
