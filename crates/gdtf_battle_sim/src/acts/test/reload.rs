use super::support::*;

fn empty_magazine(size: u16, reload_tu: u8) -> Magazine {
    Magazine::new(
        LoadedRounds::new(0),
        MagazineSize::new(size),
        ReloadTu::new(reload_tu),
    )
}

fn spawn_reload_actor(
    app: &mut App,
    magazine: Magazine,
    tu: u8,
    life: LifeState,
) -> (Entity, Entity) {
    let actor = app.world_mut().spawn((Tu::new(tu), life)).id();
    let weapon = app
        .world_mut()
        .spawn((WieldedBy::new(actor), magazine))
        .id();
    (actor, weapon)
}

fn weapon_rounds(app: &App, weapon: Entity) -> Option<u16> {
    app.world().get::<Magazine>(weapon).map(|m| *m.rounds())
}

fn drain_reload_results(app: &mut App) -> Vec<ReloadResult> {
    app.world_mut()
        .resource_mut::<Messages<ReloadResult>>()
        .drain()
        .collect()
}

#[test]
fn reload_dispatch_spends_reload_tu_and_refills_to_full() {
    let mut app = headless_app();
    let reload_cost = 12u8;
    let size = 30u16;
    let (actor, weapon) = spawn_reload_actor(
        &mut app,
        empty_magazine(size, reload_cost),
        60,
        LifeState::Alive,
    );
    let tu_before = app.world().get::<Tu>(actor).map(|t| **t);
    let mag_before = weapon_rounds(&app, weapon);
    assert_eq!(
        mag_before,
        Some(0),
        "precondition: the magazine starts empty"
    );

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    assert_eq!(
        drain_reload_results(&mut app),
        vec![ReloadResult::new(actor, ReloadOutcome::Reloaded)],
        "a successful reload must emit one ReloadResult::Reloaded for the actor",
    );

    let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
    let mag_after = weapon_rounds(&app, weapon);

    assert_eq!(
        mag_after,
        Some(size),
        "a successful reload refills the magazine to its full size",
    );
    assert!(
        matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
        "an affordable reload must strictly decrease Tu",
    );
    assert_eq!(
        tu_before.zip(tu_after).map(|(b, a)| b - a),
        Some(reload_cost),
        "the Tu drop must equal exactly the per-weapon reload_tu",
    );
}

#[test]
fn reload_dispatch_unaffordable_is_a_no_op() {
    let mut app = headless_app();
    let reload_cost = 12u8;
    let size = 30u16;
    let pool = reload_cost - 1;
    let (actor, weapon) = spawn_reload_actor(
        &mut app,
        empty_magazine(size, reload_cost),
        pool,
        LifeState::Alive,
    );

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    assert_eq!(
        drain_reload_results(&mut app),
        vec![ReloadResult::new(actor, ReloadOutcome::NoTu)],
        "an unaffordable reload must emit one ReloadResult::NoTu for the actor",
    );

    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(pool),
        "an unaffordable reload must NOT spend any TU",
    );
    assert_eq!(
        weapon_rounds(&app, weapon),
        Some(0),
        "an unaffordable reload must NOT refill the magazine",
    );
}

#[test]
fn reload_dispatch_when_not_alive_is_a_no_op() {
    let mut app = headless_app();
    let reload_cost = 12u8;
    let size = 30u16;
    for life in [LifeState::Downed, LifeState::Dead] {
        let (actor, weapon) =
            spawn_reload_actor(&mut app, empty_magazine(size, reload_cost), 60, life);
        app.world_mut().write_message(ReloadRequested::new(actor));
        app.update();
        assert_eq!(
            drain_reload_results(&mut app),
            vec![],
            "a {life:?} actor's reload must emit NO ReloadResult",
        );
        assert_eq!(
            app.world().get::<Tu>(actor).map(|t| **t),
            Some(60),
            "a {life:?} actor's reload must spend no TU",
        );
        assert_eq!(
            weapon_rounds(&app, weapon),
            Some(0),
            "a {life:?} actor's reload must not refill the magazine",
        );
    }
}

#[test]
fn reload_charges_exactly_reload_tu_cost_and_can_reload_agrees_with_the_dispatch() {
    let mut app = headless_app();
    let reload_cost = 17u8;
    let size = 30u16;
    let magazine = empty_magazine(size, reload_cost);
    let quoted = reload_tu_cost(&magazine);
    assert_eq!(
        *quoted, reload_cost,
        "reload_tu_cost must read the per-weapon ReloadTu leaf",
    );

    let (actor, _) = spawn_reload_actor(&mut app, magazine, 60, LifeState::Alive);
    assert!(
        *can_reload(LifeState::Alive, &Tu::new(60), &magazine),
        "an alive actor with an ample pool and room in the magazine can reload",
    );
    let tu_before = app.world().get::<Tu>(actor).map(|tu| **tu);

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();
    assert_eq!(
        drain_reload_results(&mut app),
        vec![ReloadResult::new(actor, ReloadOutcome::Reloaded)],
        "the reload succeeded, so the charge below is the success charge",
    );

    assert_eq!(
        tu_before
            .zip(app.world().get::<Tu>(actor).map(|tu| **tu))
            .map(|(b, a)| b - a),
        Some(*quoted),
        "the reload must charge exactly what reload_tu_cost quoted",
    );

    let broke = Tu::new(reload_cost.saturating_sub(1));
    assert!(
        !*can_reload(LifeState::Alive, &broke, &magazine),
        "can_reload must refuse the same short pool the dispatch reports as NoTu",
    );
    assert!(
        !*can_reload(LifeState::Dead, &Tu::new(60), &magazine),
        "a dead actor cannot reload however full the pool",
    );
}

#[test]
fn reload_dispatch_of_a_full_magazine_is_a_no_op_no_charge() {
    let mut app = headless_app();
    let reload_cost = 12u8;
    let size = 30u16;
    let (actor, weapon) = spawn_reload_actor(
        &mut app,
        Magazine::loaded(MagazineSize::new(size), ReloadTu::new(reload_cost)),
        60,
        LifeState::Alive,
    );

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    assert_eq!(
        drain_reload_results(&mut app),
        vec![ReloadResult::new(actor, ReloadOutcome::AlreadyFull)],
        "an already-full reload must emit one ReloadResult::AlreadyFull for the actor",
    );

    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(60),
        "reloading an already-full magazine must charge no TU (FLAGGED no-op choice)",
    );
    assert_eq!(
        weapon_rounds(&app, weapon),
        Some(size),
        "an already-full magazine stays full",
    );
}
