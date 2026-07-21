//! T4 / T7 — the spawn-flood cure, and clause (d) asserted rather than assumed.

use gdtf_battle_sim::{
    battle::{BattleLost, BattleWon},
    ganger::Direction,
    test_support::SituationBuilder,
};

use super::harness::*;

/// **T4 — THE SPAWN-FLOOD CURE.** Setting up a battle must append NOTHING to the log.
///
/// Situation setup writes facing, stance, aiming, position and life state on every ganger
/// at spawn. A recorder that keyed on `Changed<T>` — the obvious implementation — would
/// therefore fire for every one of those, for every ganger, on the spawn frame: a roster of
/// six would open its battle with dozens of entries before anybody had acted, and a
/// presenter replaying them would spend seconds showing a battle that had not started.
///
/// The cure is that a query-sourced recorder emits only on a TRANSITION against a
/// prior-value map, and a FIRST observation seeds the map silently. This test is what keeps
/// that rule from being quietly dropped.
#[test]
fn spawning_a_roster_appends_no_entries() {
    let mut app = battle_app(forced_reaction_tuning(1));

    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(ground(5, 4), PLAYER, Direction::East),
            watcher(ground(5, 6), PLAYER, Direction::East),
            tough_mover(ground(9, 5), ENEMY, Direction::West),
            tough_mover(ground(9, 7), ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let entries = logged(&app);
    assert!(
        entries.is_empty(),
        "spawning a roster must record NOTHING — every ganger's spawn-time facing / stance \
         / aim / position / life state is a FIRST observation, which seeds the transition \
         map and emits no entry. Got: {entries:?}",
    );
}

/// **T7 — CLAUSE (d), ASSERTED.** A battle with no presenter anywhere runs to an outcome.
///
/// The entire pacing mechanism lives in the presenter, and the input gate it drives fails
/// OPEN when there is no cursor. This is the test that the sim, on its own, is completely
/// unaffected: no cursor exists, nothing waits, and the battle reaches a decided outcome
/// exactly as it always did. It is the guard on the single most dangerous line in the
/// change — the fail-open default.
#[test]
fn a_battle_with_no_presenter_runs_to_an_outcome() {
    let mut app = battle_app(forced_reaction_tuning(8));
    app.init_resource::<Outcomes>();
    app.add_systems(bevy::app::Update, record_outcomes);

    // One frail enemy against a watcher that will interrupt it to death as it advances.
    let watch = ground(5, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            frail_mover(ground(8, 5), ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(mover) = ganger_at(&mut app, ground(8, 5)) else {
        unreachable!("setup spawns the mover at its fixture cell");
    };
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::MoveRequested::new(
            mover,
            ground(6, 5),
        ));

    // Run a generous number of updates. With no presenter there is nothing to wait for, so
    // the battle resolves at full speed.
    for _ in 0..64 {
        app.update();
    }

    let decided = app
        .world()
        .get_resource::<Outcomes>()
        .is_some_and(|outcomes| outcomes.decided);
    assert!(
        decided,
        "a battle with NO presenter must reach a decided outcome — nothing in the sim waits \
         on a cursor that does not exist",
    );
    // And the log still recorded the acts, so a headless run is fully observable.
    assert!(
        !logged(&app).is_empty(),
        "the recorder runs headless too — the log is what a QA client and the log tests read",
    );
}

/// A frail mover: low Grit / Toughness, so a couple of interrupt rounds finish it.
fn frail_mover(
    at: gdtf_battle_sim::metric::CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    use gdtf_battle_sim::{
        ganger::{Cool, Facing, Grit, Reflexes, Speed, Toughness},
        prelude::{Faction, Stance, StanceKind},
        test_support::GangerSpawnBuilder,
    };
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .reflexes(Reflexes::new(1.0))
        .cool(Cool::new(1.0))
        .grit(Grit::new(1.0))
        .toughness(Toughness::new(1.0))
        .build()
}

/// Whether the census has declared an outcome at any point in the run.
#[derive(bevy::prelude::Resource, Default)]
struct Outcomes {
    /// Set once a `BattleWon` or `BattleLost` has been observed.
    decided: bool,
}

/// Latch the census outcome so the assertion can read it after the run.
fn record_outcomes(
    mut won: bevy::prelude::MessageReader<BattleWon>,
    mut lost: bevy::prelude::MessageReader<BattleLost>,
    mut outcomes: bevy::prelude::ResMut<Outcomes>,
) {
    if won.read().next().is_some() || lost.read().next().is_some() {
        outcomes.decided = true;
    }
}
