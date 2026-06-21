//! GTW-275 AC3 — the reload dispatch act: an affordable, alive actor's reload spends
//! the per-weapon `reload_tu` and refills the magazine to full; an unaffordable or
//! dead actor's reload is a no-op (no TU spent, no refill). Driven through the REAL
//! act path (write `ReloadRequested`, `app.update()`), never by hand-mutating the
//! magazine. The `reload_tu` magnitude is tunable, so tests assert the spend/refill
//! RELATION, never a pinned cost.

use super::support::*;

/// An empty magazine carrying a known capacity + per-weapon reload cost — the reload
/// fixture. The `reload_tu` value is arbitrary (tunable); tests relate the TU drop to
/// it, never pin it.
fn empty_magazine(size: u16, reload_tu: u8) -> Magazine {
    Magazine::new(0, MagazineSize::new(size), ReloadTu::new(reload_tu))
}

/// Spawn a reload-act actor — the ganger carries the `Tu`/`LifeState` the gate reads, and
/// its `magazine` rides on a related **weapon entity** (`Wields`, GTW-323 slice 2: the
/// `Magazine` lives on the weapon now). The `WieldedBy` insert hook populates the
/// ganger's `Wields` synchronously in a bare `World` spawn so the next dispatch resolves
/// it. Returns `(actor, weapon)` so a test can read the magazine off the weapon entity.
fn spawn_reload_actor(
    app: &mut App,
    magazine: Magazine,
    tu: u8,
    life: LifeState,
) -> (Entity, Entity) {
    let actor = app.world_mut().spawn((Tu::new(tu), life)).id();
    let weapon = app.world_mut().spawn((WieldedBy(actor), magazine)).id();
    (actor, weapon)
}

/// The loaded round count of the actor's wielded-weapon magazine (`Wields`, GTW-323
/// slice 2) — read off the weapon entity, the authoritative ammo location.
fn weapon_rounds(app: &App, weapon: Entity) -> Option<u16> {
    app.world().get::<Magazine>(weapon).map(|m| *m.rounds())
}

/// Drain the buffered [`ReloadResult`] messages from the app's world (GTW-312) — the
/// presenter-visible reload-result signals `dispatch_reload` emitted this update.
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
    // GTW-323 slice 2: the magazine lives on the actor's wielded-weapon entity.
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

    // GTW-312: the success branch emits a single Reloaded result for this actor.
    assert_eq!(
        drain_reload_results(&mut app),
        vec![ReloadResult::new(actor, ReloadOutcome::Reloaded)],
        "a successful reload must emit one ReloadResult::Reloaded for the actor",
    );

    let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
    let mag_after = weapon_rounds(&app, weapon);

    // The magazine refilled to its full capacity.
    assert_eq!(
        mag_after,
        Some(size),
        "a successful reload refills the magazine to its full size",
    );
    // The TU drop equals exactly the magazine's own per-weapon reload_tu (a relation to
    // the weapon value, never a pinned magnitude).
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
    // The pool is one TU short of the reload cost → cannot afford → silent reject.
    let pool = reload_cost - 1;
    let (actor, weapon) = spawn_reload_actor(
        &mut app,
        empty_magazine(size, reload_cost),
        pool,
        LifeState::Alive,
    );

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    // GTW-312: the can't-afford branch emits a single NoTu result for this actor.
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
        // GTW-312: the not-alive guard is an INTERNAL skip — it emits NO ReloadResult
        // (a dead/absent ganger never issues a reload intent).
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
fn reload_dispatch_of_a_full_magazine_is_a_no_op_no_charge() {
    // FLAGGED behavior (GTW-275 AC3): reloading an already-FULL magazine is a no-op
    // that charges NO TU (mirrors the codebase's "redundant act = no charge" rule).
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

    // GTW-312: the already-full branch emits a single AlreadyFull result for this actor.
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
