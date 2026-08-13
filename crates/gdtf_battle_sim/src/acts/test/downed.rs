use super::support::*;

// The price the sim itself quotes for a downed act, read from the running world.
fn quoted_cost(app: &App, price: fn(&CombatTuning) -> Tu) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), price)
}

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

#[test]
fn execute_dispatch_debits_the_actor_pool_by_the_sims_own_quote() {
    let mut app = headless_app();
    let cost = quoted_cost(&app, execute_tu_cost);
    assert!(
        *cost > 0,
        "an execute must be priced or no debit is visible"
    );

    let before = Tu::new((*cost).saturating_add(10));
    let actor = spawn_downed_actor_with_pool(app.world_mut(), 10, 10, 1, before);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 2);

    app.world_mut()
        .write_message(ExecuteDownedRequested::new(actor, target));
    app.update();

    assert_eq!(
        app.world().get::<Tu>(actor).copied(),
        Some(Tu::new((*before).saturating_sub(*cost))),
        "an execute the sim carried out must debit the actor's pool by execute_tu_cost",
    );
    assert_eq!(
        app.world().get::<LifeState>(target).copied(),
        Some(LifeState::Dead),
        "the charged execute still kills the target",
    );
}

#[test]
fn an_execute_pool_one_short_of_the_quote_refuses_the_act() {
    let mut app = headless_app();
    let cost = quoted_cost(&app, execute_tu_cost);
    assert!(
        *cost > 0,
        "an execute must be priced or no short pool exists"
    );

    let before = Tu::new((*cost).saturating_sub(1));
    let actor = spawn_downed_actor_with_pool(app.world_mut(), 10, 10, 1, before);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 2);

    app.world_mut()
        .write_message(ExecuteDownedRequested::new(actor, target));
    app.update();

    assert_eq!(
        app.world().get::<LifeState>(target).copied(),
        Some(LifeState::Downed),
        "a pool one short of execute_tu_cost refuses the act — the target stays Downed",
    );
    assert_eq!(
        app.world().get::<Tu>(actor).copied(),
        Some(before),
        "a refused execute spends nothing — spend_tu never clamps",
    );
}

#[test]
fn an_execute_pool_of_exactly_the_quote_carries_the_act_and_lands_on_zero() {
    let mut app = headless_app();
    let cost = quoted_cost(&app, execute_tu_cost);
    assert!(
        *cost > 0,
        "an execute must be priced or a pool of exactly the quote is an empty pool"
    );

    let actor = spawn_downed_actor_with_pool(app.world_mut(), 10, 10, 1, cost);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 2);

    app.world_mut()
        .write_message(ExecuteDownedRequested::new(actor, target));
    app.update();

    assert_eq!(
        app.world().get::<LifeState>(target).copied(),
        Some(LifeState::Dead),
        "a pool of exactly execute_tu_cost affords the act — the target must reach Dead",
    );
    assert_eq!(
        app.world().get::<Tu>(actor).copied(),
        Some(Tu::new(0)),
        "the carried execute debits the whole pool — the actor lands on zero",
    );
}

#[test]
fn stabilize_dispatch_debits_the_actor_pool_by_the_sims_own_quote() {
    let mut app = headless_app();
    let cost = quoted_cost(&app, stabilize_tu_cost);
    assert!(
        *cost > 0,
        "a stabilize must be priced or no debit is visible"
    );

    let before = Tu::new((*cost).saturating_add(10));
    let actor = spawn_downed_actor_with_pool(app.world_mut(), 10, 10, 1, before);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 1);

    app.world_mut()
        .write_message(StabilizeDownedRequested::new(actor, target));
    app.update();

    assert_eq!(
        app.world().get::<Tu>(actor).copied(),
        Some(Tu::new((*before).saturating_sub(*cost))),
        "a stabilize the sim carried out must debit the actor's pool by stabilize_tu_cost",
    );
    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "the charged stabilize still removes the BleedingOut condition",
    );
}

#[test]
fn a_stabilize_pool_one_short_of_the_quote_refuses_the_act() {
    let mut app = headless_app();
    let cost = quoted_cost(&app, stabilize_tu_cost);
    assert!(
        *cost > 0,
        "a stabilize must be priced or no short pool exists"
    );

    let before = Tu::new((*cost).saturating_sub(1));
    let actor = spawn_downed_actor_with_pool(app.world_mut(), 10, 10, 1, before);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 1);

    app.world_mut()
        .write_message(StabilizeDownedRequested::new(actor, target));
    app.update();

    assert!(
        app.world().get::<BleedingOut>(target).is_some(),
        "a pool one short of stabilize_tu_cost refuses the act — the condition stays",
    );
    assert_eq!(
        app.world().get::<Tu>(actor).copied(),
        Some(before),
        "a refused stabilize spends nothing — spend_tu never clamps",
    );
}

#[test]
fn a_stabilize_pool_of_exactly_the_quote_carries_the_act_and_lands_on_zero() {
    let mut app = headless_app();
    let cost = quoted_cost(&app, stabilize_tu_cost);
    assert!(
        *cost > 0,
        "a stabilize must be priced or a pool of exactly the quote is an empty pool"
    );

    let actor = spawn_downed_actor_with_pool(app.world_mut(), 10, 10, 1, cost);
    let target = spawn_downed_target(app.world_mut(), 11, 10, 1);

    app.world_mut()
        .write_message(StabilizeDownedRequested::new(actor, target));
    app.update();

    assert!(
        app.world().get::<BleedingOut>(target).is_none(),
        "a pool of exactly stabilize_tu_cost affords the act — the condition must be removed",
    );
    assert_eq!(
        app.world().get::<Tu>(actor).copied(),
        Some(Tu::new(0)),
        "the carried stabilize debits the whole pool — the actor lands on zero",
    );
}
