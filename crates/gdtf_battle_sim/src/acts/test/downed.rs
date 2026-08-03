use super::support::*;

#[test]
fn stabilize_dispatch_from_adjacent_ally_removes_the_condition_and_keeps_downed() {
    let mut app = headless_app();
    let actor = spawn_downed_actor(app.world_mut(), 10, 10, 1);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 1); 

    app.world_mut()
        .write_message(StabilizeDownedRequested::new(actor, target));
    app.update();

    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "an adjacent alive ALLY's stabilize dispatch must REMOVE the BleedingOut condition",
    );
    assert_eq!(
        app.world().get::<LifeState>(target).copied(),
        Some(LifeState::Downed),
        "stabilize never writes LifeState — the target stays Downed",
    );
}

#[test]
fn execute_dispatch_from_adjacent_enemy_kills_the_target() {
    let mut app = headless_app();
    let actor = spawn_downed_actor(app.world_mut(), 10, 10, 1);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 2); 

    app.world_mut()
        .write_message(ExecuteDownedRequested::new(actor, target));
    app.update();

    assert_eq!(
        app.world().get::<LifeState>(target).copied(),
        Some(LifeState::Dead),
        "an adjacent alive ENEMY's execute dispatch must transition the target to Dead",
    );
}

#[test]
fn faction_gate_holds_end_to_end_through_dispatch() {
    let mut app = headless_app();
    let enemy_actor = spawn_downed_actor(app.world_mut(), 10, 10, 1);
    let downed_enemy = spawn_downed_target(app.world_mut(), 11, 10, 2); 
    app.world_mut()
        .write_message(StabilizeDownedRequested::new(enemy_actor, downed_enemy));
    app.update();
    assert!(
        app.world().get::<BleedingOut>(downed_enemy).is_some(),
        "a cross-faction (enemy) stabilize dispatch is a no-op — the BleedingOut condition stays",
    );

    let mut app2 = headless_app();
    let ally_actor = spawn_downed_actor(app2.world_mut(), 10, 10, 1);
    let downed_ally = spawn_downed_target(app2.world_mut(), 11, 10, 1); 
    app2.world_mut()
        .write_message(ExecuteDownedRequested::new(ally_actor, downed_ally));
    app2.update();
    assert_eq!(
        app2.world().get::<LifeState>(downed_ally).copied(),
        Some(LifeState::Downed),
        "a same-faction (ally) execute dispatch is a no-op — the target stays Downed",
    );
}
