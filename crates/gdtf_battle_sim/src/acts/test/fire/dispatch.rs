//! AC3 — the core `FireRequested` dispatch runs `fire()` and mutates the target,
//! plus seed determinism on the dispatch path.

use super::support::*;

#[test]
fn fire_dispatch_runs_fire_and_mutates_the_target() {
    let (mut app, shooter, target) = fire_scenario();
    let mode = single_mode(0.2, 1);
    let hp_before = app.world().get::<Hp>(target).copied();
    let tu_before = app.world().get::<Tu>(shooter).copied();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).map(|w| **w);
    let life_after = app.world().get::<LifeState>(target).copied();
    let tu_after = app.world().get::<Tu>(shooter).copied();

    // The verb RAN: either the target's battle surfaces changed (a real shot effect)
    // OR (seed-dependent miss) the shooter's Tu strictly decreased (the charge). Never
    // a pinned magnitude.
    let target_changed =
        hp_after != hp_before || wounds_after != Some(6) || life_after != Some(LifeState::Alive);
    let tu_dropped = matches!((tu_before, tu_after), (Some(b), Some(a)) if *a < *b);
    assert!(
        target_changed || tu_dropped,
        "fire dispatch must run the verb — target surfaces changed or the shooter's \
         Tu dropped (hp {hp_before:?}->{hp_after:?}, tu {tu_before:?}->{tu_after:?})",
    );
}

#[test]
fn fire_dispatch_is_deterministic_for_the_same_seed() {
    let snapshot = |seed_run: u64| {
        // The seed is fixed by insert_sim_resources; seed_run only labels the call.
        let _ = seed_run;
        let (mut app, shooter, target) = fire_scenario();
        let mode = single_mode(0.2, 1);
        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();
        (
            app.world().get::<Hp>(target).map(|h| **h),
            app.world().get::<Wounds>(target).map(|w| **w),
            app.world().get::<LifeState>(target).copied(),
            app.world().get::<Tu>(shooter).map(|t| **t),
        )
    };
    assert_eq!(
        snapshot(0),
        snapshot(1),
        "the same BattleSeed must reproduce the same post-fire state via dispatch",
    );
}
