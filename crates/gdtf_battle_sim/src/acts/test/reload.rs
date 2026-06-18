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

#[test]
fn reload_dispatch_spends_reload_tu_and_refills_to_full() {
    let mut app = headless_app();
    let reload_cost = 12u8;
    let size = 30u16;
    let actor = app
        .world_mut()
        .spawn((
            empty_magazine(size, reload_cost),
            Tu::new(60),
            LifeState::Alive,
        ))
        .id();
    let tu_before = app.world().get::<Tu>(actor).map(|t| **t);
    let mag_before = app.world().get::<Magazine>(actor).map(|m| *m.rounds());
    assert_eq!(
        mag_before,
        Some(0),
        "precondition: the magazine starts empty"
    );

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
    let mag_after = app.world().get::<Magazine>(actor).map(|m| *m.rounds());

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
    let actor = app
        .world_mut()
        .spawn((
            empty_magazine(size, reload_cost),
            Tu::new(pool),
            LifeState::Alive,
        ))
        .id();

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(pool),
        "an unaffordable reload must NOT spend any TU",
    );
    assert_eq!(
        app.world().get::<Magazine>(actor).map(|m| *m.rounds()),
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
        let actor = app
            .world_mut()
            .spawn((empty_magazine(size, reload_cost), Tu::new(60), life))
            .id();
        app.world_mut().write_message(ReloadRequested::new(actor));
        app.update();
        assert_eq!(
            app.world().get::<Tu>(actor).map(|t| **t),
            Some(60),
            "a {life:?} actor's reload must spend no TU",
        );
        assert_eq!(
            app.world().get::<Magazine>(actor).map(|m| *m.rounds()),
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
    let actor = app
        .world_mut()
        .spawn((
            Magazine::loaded(MagazineSize::new(size), ReloadTu::new(reload_cost)),
            Tu::new(60),
            LifeState::Alive,
        ))
        .id();

    app.world_mut().write_message(ReloadRequested::new(actor));
    app.update();

    assert_eq!(
        app.world().get::<Tu>(actor).map(|t| **t),
        Some(60),
        "reloading an already-full magazine must charge no TU (FLAGGED no-op choice)",
    );
    assert_eq!(
        app.world().get::<Magazine>(actor).map(|m| *m.rounds()),
        Some(size),
        "an already-full magazine stays full",
    );
}
