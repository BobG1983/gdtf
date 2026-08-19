use super::support::*;
use crate::{
    effects::{
        bleed::{BleedingOut, tick_bleed},
        dot::tick_dot,
        fields::{FieldDamage, FieldDef, FieldDuration, ImmuneArmorTypes, tick_fields},
    },
    ganger::Wounds,
    test_support::dot_turns,
    tuning::CombatTuning,
    weapon::{Dot, DotDamage},
};

#[test]
fn tick_dot_emits_on_death_when_a_dot_kills() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<crate::effects::dot::DotTicked>();
    app.add_message::<OnDeathOccurred>();
    app.init_resource::<CapturedDeaths>();
    app.add_systems(Update, (tick_dot, capture_deaths).chain());

    let cell = ground(4, 4);
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(3),
            LifeState::Alive,
            Position::new(cell),
            Dot {
                remaining_turns: dot_turns(2),
                per_turn_damage: DotDamage::new(5),
                damage_type:     DamageType::Plasma,
            },
        ))
        .id();
    app.update();

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the DOT killed the ganger"
    );
    assert!(
        captured_death_at(&app, ganger, cell),
        "tick_dot emits OnDeathOccurred at the dead ganger's cell on a DOT-kill"
    );
}

#[test]
fn tick_fields_emits_on_death_when_a_field_kills() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<crate::effects::fields::FieldTicked>();
    app.add_message::<OnDeathOccurred>();
    app.add_message::<crate::effects::fields::FieldAfflicted>();
    app.init_resource::<CapturedDeaths>();
    app.add_systems(Update, (tick_fields, capture_deaths).chain());

    let cell = ground(3, 3);
    let ganger = app
        .world_mut()
        .spawn((Hp::new(2), LifeState::Alive, crate::armor::Wears::default()))
        .id();
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(
        cell,
        FieldDef::new(
            FieldDamage::new(5),
            DamageType::Chem,
            ImmuneArmorTypes::default(),
            FieldDuration::Permanent,
        ),
    );
    app.world_mut().insert_resource(registry);
    app.update();

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the field killed the occupant"
    );
    assert!(
        captured_death_at(&app, ganger, cell),
        "tick_fields emits OnDeathOccurred at the field cell on a field-kill"
    );
}

#[test]
fn tick_bleed_emits_on_death_when_the_wounds_bleed_out_kills() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<crate::effects::bleed::Bleeding>();
    app.add_message::<OnDeathOccurred>();
    app.add_message::<crate::effects::bleed::BleedStarted>();
    app.init_resource::<CapturedDeaths>();
    app.insert_resource(CombatTuning {
        bleed_rate: crate::tuning::BleedRate::new(5),
        ..Default::default()
    });
    app.add_systems(Update, (tick_bleed, capture_deaths).chain());

    let cell = ground(2, 2);
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(10),
            Wounds::new(1),
            LifeState::Downed,
            BleedingOut,
            Position::new(cell),
        ))
        .id();
    app.update();

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the Wounds bleed-out killed the Downed ganger"
    );
    assert!(
        captured_death_at(&app, ganger, cell),
        "tick_bleed emits OnDeathOccurred at the dead ganger's cell on a bleed-out kill"
    );
}
