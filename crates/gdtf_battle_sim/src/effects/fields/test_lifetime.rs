//! GTW-659 — the `Turns(n)` LIFETIME pin on the REAL schedule: an authored
use bevy::prelude::{App, Entity, Messages, MinimalPlugins};

use super::{FieldDamage, FieldDef, FieldDuration, FieldRegistry, FieldTicked, ImmuneArmorTypes};
use crate::{
    acts::EndTurnRequested,
    armor::{ArmorType, WornBy},
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    ganger::{Faction, Hp, LifeState},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, field_turns, full_vision, insert_sim_resources},
    turn::ActiveFaction,
    weapon::DamageType,
};

const SEED: u64 = 0x659F_1E1D;

const PLAYER: Faction = Faction::new(0);
const ENEMY: Faction = Faction::new(1);

const PER_TURN: u16 = 3;
const START_HP: u16 = 100;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn finite_field(turns: u8) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(PER_TURN),
        DamageType::Chem,
        ImmuneArmorTypes::new([]),
        FieldDuration::Turns(field_turns(turns)),
    )
}

fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(full_vision());
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.insert_resource(BattleInProgress);
    app
}

fn stand_ganger_on(app: &mut App, cell: CellLevel) -> Entity {
    let occupant = GangerEntityBuilder::new()
        .at(cell)
        .faction(ENEMY)
        .life_state(LifeState::Alive)
        .hp(START_HP)
        .tu(100)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut()
        .spawn((ArmorType::Plated, WornBy::new(occupant)));
    app.update();
    app.update();
    occupant
}

fn cross_one_round(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
    app.update();
}

fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

fn field_count(app: &App) -> usize {
    app.world()
        .get_resource::<FieldRegistry>()
        .map_or(0, FieldRegistry::len)
}

fn drain_field_ticks(app: &mut App) -> Vec<FieldTicked> {
    app.world_mut()
        .resource_mut::<Messages<FieldTicked>>()
        .drain()
        .collect()
}

#[test]
fn a_turns_one_field_drains_exactly_one_round_then_is_gone() {
    let cell = ground(10, 10);
    let mut app = live_app();
    let occupant = stand_ganger_on(&mut app, cell);

    let mut registry = FieldRegistry::new();
    registry.spawn(cell, finite_field(1));
    app.insert_resource(registry);
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "no FieldTicked may be emitted before a round boundary",
    );

    cross_one_round(&mut app);
    let ticks = drain_field_ticks(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "Turns(1): the first round boundary drains exactly once: {ticks:?}",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - PER_TURN,
        "Turns(1): exactly one per-turn drain lands",
    );
    assert_eq!(
        field_count(&app),
        0,
        "Turns(1): the boundary that spends the last turn REMOVES the placement",
    );

    cross_one_round(&mut app);
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "Turns(1) must NOT drain a second round (the authored count is the round count)",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - PER_TURN,
        "Turns(1): the total drain stays one per-turn amount",
    );
}

#[test]
fn a_turns_two_field_drains_exactly_two_rounds_then_is_gone() {
    let cell = ground(12, 12);
    let mut app = live_app();
    let occupant = stand_ganger_on(&mut app, cell);

    let mut registry = FieldRegistry::new();
    registry.spawn(cell, finite_field(2));
    app.insert_resource(registry);

    cross_one_round(&mut app);
    assert_eq!(
        drain_field_ticks(&mut app).len(),
        1,
        "Turns(2): the first round boundary drains once",
    );
    assert_eq!(
        field_count(&app),
        1,
        "Turns(2): one turn remains after the first round — the field is still live",
    );

    cross_one_round(&mut app);
    assert_eq!(
        drain_field_ticks(&mut app).len(),
        1,
        "Turns(2): the second round boundary drains once more",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - 2 * PER_TURN,
        "Turns(2): exactly two per-turn drains land in total",
    );
    assert_eq!(
        field_count(&app),
        0,
        "Turns(2): the second boundary REMOVES the placement",
    );

    cross_one_round(&mut app);
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "Turns(2) must NOT drain a third round (the authored count is the round count)",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - 2 * PER_TURN,
        "Turns(2): the total drain stays two per-turn amounts",
    );
}
