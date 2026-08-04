use bevy::prelude::{App, Messages, MinimalPlugins};

use super::support::{dot, ground, hp_of, life_of};
use crate::{
    acts::{EndTurnRequested, MoveRequested, MovementOccurred},
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    effects::dot::DotTicked,
    ganger::{Faction, LifeState, Position},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, full_vision, insert_sim_resources},
    turn::ActiveFaction,
};

const SEED: u64 = 0x5A1C_AC75;

const PLAYER: Faction = Faction::new(0);
const ENEMY: Faction = Faction::new(1);

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

fn drain_dot_ticks(app: &mut App) -> Vec<DotTicked> {
    app.world_mut()
        .resource_mut::<Messages<DotTicked>>()
        .drain()
        .collect()
}

fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

#[test]
fn the_boundary_dot_tick_resolves_before_a_same_frame_enemy_act() {
    let per_turn = 5u16;
    let start_hp = per_turn + 10; 
    let origin = ground(10, 10);
    let dest = ground(11, 10);

    let mut app = live_app();
    app.insert_resource(full_vision());

    let mover = GangerEntityBuilder::new()
        .at(origin)
        .faction(ENEMY)
        .life_state(LifeState::Alive)
        .hp(start_hp)
        .tu(100)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut().entity_mut(mover).insert(dot(per_turn, 3));

    app.update();
    app.update();
    assert_eq!(
        hp_of(&app, mover),
        start_hp,
        "plain mid-turn frames must not tick the DOT clock (no turn boundary crossed)",
    );
    assert!(
        drain_dot_ticks(&mut app).is_empty(),
        "no DotTicked may be emitted mid-turn",
    );

    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    end_turn(&mut app);

    let ticks = drain_dot_ticks(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "the boundary frame ticks the DOT clock exactly once: {ticks:?}",
    );
    assert!(
        ticks.iter().all(|tick| tick.at == origin),
        "the boundary DotTicked lands at the mover's TURN-START cell — the clock \
         resolves BEFORE the same-frame act's step: {ticks:?}",
    );
    assert_eq!(
        hp_of(&app, mover),
        start_hp - per_turn,
        "the boundary tick drains exactly one per-turn amount",
    );
    assert_eq!(
        life_of(&app, mover),
        LifeState::Alive,
        "the non-lethal boundary tick leaves the mover Alive",
    );

    let movements = drain_movements(&mut app);
    assert_eq!(
        movements.len(),
        1,
        "the same-frame enemy move must actually RESOLVE (one MovementOccurred) — \
         otherwise this pin proves nothing: {movements:?}",
    );
    let at = app.world().get::<Position>(mover).map(|p| **p);
    assert_eq!(
        at,
        Some(dest),
        "the mover ends the boundary frame at the destination (the act resolved \
         AFTER the clock)",
    );
}
