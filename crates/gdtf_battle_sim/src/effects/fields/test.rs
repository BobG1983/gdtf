use bevy::prelude::{
    App, Deref, DerefMut, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut,
    Resource, Update,
};

use super::{
    FieldDamage, FieldDef, FieldDuration, FieldRegistry, FieldTicked, ImmuneArmorTypes, tick_fields,
};
use crate::{
    armor::{ArmorType, WornBy},
    ganger::{Hp, LifeState},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    test_support::field_turns,
    weapon::DamageType,
};

#[derive(Resource, Default, Deref, DerefMut)]
struct Captured(Vec<FieldTicked>);

fn consume(mut reader: MessageReader<FieldTicked>, mut captured: ResMut<Captured>) {
    for ticked in reader.read() {
        captured.push(*ticked);
    }
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn tick_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<FieldTicked>();
    app.add_message::<crate::effects::on_death::OnDeathOccurred>();
    app.add_message::<crate::effects::fields::FieldAfflicted>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_fields, consume).chain());
    app
}

fn field(damage: u16, immune: &[ArmorType], duration: FieldDuration) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(damage),
        DamageType::Chem,
        ImmuneArmorTypes::new(immune.iter().copied()),
        duration,
    )
}

fn grid_with_occupant(app: &mut App, cell: CellLevel, occupant: Entity) {
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(cell, Some(occupant));
    app.world_mut().insert_resource(grid);
}

fn spawn_armored_ganger(app: &mut App, hp: u16, armor_type: ArmorType) -> Entity {
    let ganger = app.world_mut().spawn((Hp::new(hp), LifeState::Alive)).id();
    app.world_mut().spawn((armor_type, WornBy::new(ganger)));
    ganger
}

fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

fn field_count(app: &App) -> usize {
    app.world()
        .get_resource::<FieldRegistry>()
        .map_or(0, FieldRegistry::len)
}

fn tick_count_for(app: &App, occupant: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|t| t.occupant == occupant).count())
}


#[test]
fn field_damages_an_unprotected_occupant_each_round() {
    let cell = ground(4, 4);
    let per_turn = 3u16;
    let start_hp = 100u16;

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, start_hp, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(
        cell,
        field(per_turn, &[ArmorType::Flak], FieldDuration::Permanent),
    );
    app.world_mut().insert_resource(registry);

    let rounds = 3u16;
    for _ in 0..rounds {
        app.update();
    }
    assert_eq!(
        hp_of(&app, ganger),
        start_hp - per_turn * rounds,
        "each field round drains exactly per_turn HP (a flat direct drain, no matchup / RNG)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Alive,
        "a non-lethal field tick leaves the occupant Alive",
    );
    assert_eq!(
        tick_count_for(&app, ganger),
        usize::from(rounds),
        "a FieldTicked is emitted once per draining round",
    );
}


#[test]
fn immune_armor_skips_the_field_damage() {
    let cell = ground(2, 2);
    let start_hp = 50u16;

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, start_hp, ArmorType::Flak);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(9, &[ArmorType::Flak], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);
    app.update();

    app.update();
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        start_hp,
        "whole-source immunity: a worn ArmorType in the immune set skips ALL field damage",
    );
    assert_eq!(
        tick_count_for(&app, ganger),
        0,
        "an immune occupant emits NO FieldTicked (it took no damage)",
    );
}


#[test]
fn turns_field_counts_down_and_is_removed_after_n_rounds() {
    let cell = ground(3, 3);
    let turns = 2u8;

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, 100, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(
        cell,
        field(3, &[], FieldDuration::Turns(field_turns(turns))),
    );
    app.world_mut().insert_resource(registry);
    app.update();

    assert_eq!(field_count(&app), 1, "the field is live before any tick");
    for _ in 0..turns {
        app.update();
    }
    assert_eq!(
        field_count(&app),
        0,
        "a Turns field is removed once its turn count runs out",
    );
    let hp_after_expiry = hp_of(&app, ganger);
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        hp_after_expiry,
        "with the field removed a further round drains no HP",
    );
}


#[test]
fn permanent_field_never_expires() {
    let cell = ground(5, 5);

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, 100, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(1, &[], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);
    app.update();

    for _ in 0..10 {
        app.update();
        assert_eq!(
            field_count(&app),
            1,
            "a Permanent field never expires, however many rounds tick",
        );
    }
}


#[test]
fn a_field_tick_that_empties_hp_flips_the_occupant_to_dead() {
    let cell = ground(1, 1);

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, 8, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(10, &[], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);
    app.update();

    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        0,
        "the field tick floors HP at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "a field drain that empties HP KILLS (Dead — the DOT-kills precedent)",
    );
}


#[derive(Resource, Default, Deref, DerefMut)]
struct CapturedAfflicted(Vec<crate::effects::fields::FieldAfflicted>);

fn consume_afflicted(
    mut reader: MessageReader<crate::effects::fields::FieldAfflicted>,
    mut captured: ResMut<CapturedAfflicted>,
) {
    for afflicted in reader.read() {
        captured.push(*afflicted);
    }
}

fn afflicted_count_for(app: &App, occupant: Entity) -> usize {
    app.world()
        .get_resource::<CapturedAfflicted>()
        .map_or(0, |c| c.iter().filter(|a| a.occupant == occupant).count())
}

#[test]
fn a_field_exposure_announces_once_per_span_and_reannounces_after_leaving() {
    let cell = ground(5, 5);
    let mut app = tick_app();
    app.init_resource::<CapturedAfflicted>();
    app.add_systems(Update, consume_afflicted.after(tick_fields));
    let ganger = spawn_armored_ganger(&mut app, 100, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);

    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(3, &[], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);

    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "the FIRST draining round emits exactly one FieldAfflicted",
    );

    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "a mid-exposure round emits NO further FieldAfflicted (once per span)",
    );
    assert!(
        tick_count_for(&app, ganger) >= 2,
        "the per-round FieldTicked drain keeps firing mid-span",
    );

    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, None);
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "an off-field round emits nothing",
    );

    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        2,
        "re-entering the field is a NEW span and must re-announce",
    );
}
