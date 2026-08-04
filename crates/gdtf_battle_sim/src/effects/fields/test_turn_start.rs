use bevy::prelude::{App, Entity, Messages, MinimalPlugins};

use super::{FieldDamage, FieldDef, FieldDuration, FieldRegistry, FieldTicked, ImmuneArmorTypes};
use crate::{
    acts::{EndTurnRequested, MoveRejected, MoveRequested, MovementOccurred},
    armor::{ArmorType, WornBy},
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    ganger::{Faction, Hp, LifeState, Position},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, full_vision, insert_sim_resources},
    turn::ActiveFaction,
    weapon::DamageType,
};

const SEED: u64 = 0x5A1C_AC75;

const PLAYER: Faction = Faction::new(0);
const ENEMY: Faction = Faction::new(1);

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn field(damage: u16) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(damage),
        DamageType::Chem,
        ImmuneArmorTypes::new([]),
        FieldDuration::Permanent,
    )
}

fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.insert_resource(BattleInProgress);
    app
}

fn end_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
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

fn drain_field_ticks(app: &mut App) -> Vec<FieldTicked> {
    app.world_mut()
        .resource_mut::<Messages<FieldTicked>>()
        .drain()
        .collect()
}

fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

#[test]
fn the_boundary_field_tick_resolves_before_a_same_frame_enemy_act() {
    let per_turn = 10u16;
    let start_hp = per_turn - 2;
    let field_cell = ground(10, 10);
    let dest = ground(11, 10);

    let mut app = live_app();
    app.insert_resource(full_vision());

    let mover = GangerEntityBuilder::new()
        .at(field_cell)
        .faction(ENEMY)
        .life_state(LifeState::Alive)
        .hp(start_hp)
        .tu(100)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut()
        .spawn((ArmorType::Plated, WornBy::new(mover)));

    let mut registry = FieldRegistry::new();
    registry.spawn(field_cell, field(per_turn));
    app.insert_resource(registry);

    app.update();
    app.update();
    assert_eq!(
        hp_of(&app, mover),
        start_hp,
        "plain mid-turn frames must not tick the field clock (no turn boundary crossed)",
    );
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "no FieldTicked may be emitted mid-turn",
    );

    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    end_turn(&mut app);

    let ticks = drain_field_ticks(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "the boundary frame ticks the field clock exactly once: {ticks:?}",
    );
    assert!(
        ticks.iter().all(|tick| tick.at == field_cell),
        "the boundary FieldTicked lands at the field cell: {ticks:?}",
    );
    assert_eq!(hp_of(&app, mover), 0, "the lethal tick empties the Hp pool");
    assert_eq!(
        life_of(&app, mover),
        LifeState::Dead,
        "a field drain that empties HP KILLS (the DOT-kills precedent)",
    );
    let movements = drain_movements(&mut app);
    assert!(
        movements.is_empty(),
        "the same-frame act resolves AFTER the clock: the boundary kill means NO \
         step and NO MovementOccurred: {movements:?}",
    );
    let at = app.world().get::<Position>(mover).map(|p| **p);
    assert_eq!(
        at,
        Some(field_cell),
        "the mover dies ON the field cell — it never stepped off",
    );

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects.is_empty(),
        "the in-flight move must pass the dispatch gates (reachable + affordable) — \
         a reject would make this pin vacuous: {rejects:?}",
    );
}
