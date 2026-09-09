use bevy::prelude::{Entity, With};

use super::support::*;

fn ganger_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<Faction>>();
    query.iter(world).collect()
}

#[test]
fn teardown_despawns_the_battle_gangers_so_a_second_battle_stands_alone() {
    let mut app = headless_app();

    let first = fixtures::two_ganger();
    let first_authored = first.1.len();
    app.world_mut()
        .write_message(setup_request(first, BattleSeed::new(SEED)));
    app.update();
    app.update();

    let first_gangers = ganger_entities(&mut app);
    assert_eq!(
        first_gangers.len(),
        first_authored,
        "precondition: the first setup must spawn one ganger entity per authored ganger; found {} \
         ganger entities",
        first_gangers.len(),
    );

    app.world_mut().write_message(TeardownBattleRequested);
    app.update();

    let second = fixtures::one_player_two_enemies();
    let second_authored = second.1.len();
    app.world_mut()
        .write_message(setup_request(second, BattleSeed::new(SEED)));
    app.update();
    app.update();

    let survivors = first_gangers
        .iter()
        .filter(|&&entity| app.world().get_entity(entity).is_ok())
        .count();
    assert_eq!(
        survivors,
        0,
        "teardown must despawn every ganger of the battle it tears down; {survivors} of {} \
         first-battle ganger entities are still alive",
        first_gangers.len(),
    );

    let alive = ganger_entities(&mut app);
    assert_eq!(
        alive.len(),
        second_authored,
        "only the second battle's gangers may be alive after it sets up; found {} ganger entities",
        alive.len(),
    );
}
