//! GTW-457 — the stacked-ganger dedup-gate abort.

use super::super::support::*;

/// GTW-457 — TWO authored gangers on the SAME `(cell, level)` make `setup_battle`
/// return `Err(BattleSetupError::StackedGangers { at })` naming the shared cell, and
/// spawn NOTHING (the pre-spawn dedup gate runs BEFORE the spawn loop — the abort-first
/// invariant). Both gangers carry VALID, resolvable `(gang, member)` refs (distinct
/// factions → distinct synthesized gangs/members), so the failure is provably the
/// STACK, not a missing ref. Pin-discriminating: with the gate removed, the GTW-156
/// last-write-wins occupancy pour would let both gangers spawn STACKED on one cell —
/// so this returns `Ok` (no error) and spawns two ganger entities, reddening every
/// assertion below.
#[test]
fn setup_errors_on_stacked_gangers() {
    let shared = key(5, 5, 0);
    // Two gangers on the SAME cell, on DISTINCT factions — `build_with_gangs` synthesizes
    // a registry where BOTH resolve (distinct gang per faction, distinct member name), so
    // the gang/member resolution passes and the StackedGangers gate is the sole cause.
    // The builder does not dedupe placements, preserving the stack the gate must reject.
    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([ganger_at(shared, 0), ganger_at(shared, 1)])
        .build_with_gangs();
    // Sanity: the fixture really authors two gangers on the one cell (else the gate is
    // vacuously satisfied and the test proves nothing).
    assert_eq!(situation.gangers.len(), 2, "the fixture fields two gangers");
    assert!(
        situation.gangers.iter().all(|g| g.at == shared),
        "both gangers are authored on the one shared cell",
    );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — this
    // test aborts BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // No terrain authored (the dedup gate fires before terrain resolution),
                // so terrain: None + the fallback floor cost suffices.
                BattleRegistries::new(&gangs, &registry, &melee, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::StackedGangers { at: shared }),
        "two gangers on one cell must abort setup with StackedGangers, naming the cell",
    );

    // Nothing was spawned (the dedup gate aborted before the spawn loop) — neither ganger
    // entities (no Position) nor any resource. If the gate were removed, BOTH gangers
    // would spawn here (two Positions), so this count discriminates the gate.
    app.world_mut().flush();
    let world = app.world_mut();
    let mut positions = world.query::<&Position>();
    assert_eq!(
        positions.iter(world).count(),
        0,
        "a stacked-ganger abort must spawn no ganger entities",
    );
    let mut wears = world.query::<&Wears>();
    assert_eq!(
        wears.iter(world).count(),
        0,
        "a stacked-ganger abort must spawn no worn-armor entities",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a stacked-ganger abort must insert no resources",
    );
    assert!(
        world.get_resource::<OccupancyGrid>().is_none(),
        "a stacked-ganger abort must insert no OccupancyGrid",
    );
}
