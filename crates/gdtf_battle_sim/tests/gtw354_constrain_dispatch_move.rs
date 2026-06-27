//! GTW-354 (E7 · GTW-12f) — `dispatch_move` is the SINGLE writer that constrains a move
//! commit to a REACHABLE, AFFORDABLE path, end-to-end on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path.
//!
//! Proves the three GTW-354 C6 behaviors on the live runtime:
//!
//! 1. **A teleport to a non-adjacent UNREACHABLE cell is REJECTED** — a `MoveRequested`
//!    for a far cell the squad has never seen (UNSEEN → non-routable, GTW-353) emits a
//!    typed `MoveRejected(Unreachable)`, steps NOTHING, and announces NO `MovementOccurred`
//!    (this kills the pre-GTW-354 any-empty-cell teleport).
//! 2. **A REACHABLE route is ACCEPTED** — a `MoveRequested` for an adjacent, in-sight,
//!    affordable cell steps the mover (its `Position` changes) and announces exactly one
//!    `MovementOccurred`, with no `MoveRejected`.
//! 3. **An UNAFFORDABLE route is REJECTED with NO partial move** — a `MoveRequested` for a
//!    reachable adjacent cell when the mover has too little TU to afford the whole route
//!    emits a typed `MoveRejected(Unaffordable)`, leaves the mover exactly where it was
//!    (no partial move), and announces NO `MovementOccurred`.
//!
//! RELATIONS-ONLY: the unaffordable case sets the mover's TU to ZERO so ANY positive-cost
//! route is unaffordable (no pinned shipped move-cost magnitude); the affordable case uses
//! the default spawn TU which comfortably covers one open step.
//!
//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`"): the sim crate is
//! the LOW crate — `gdtf_test_utils` depends on `gdtf_app` which depends on
//! `gdtf_battle_sim`, so a sim-crate dev-dep on `gdtf_test_utils` would be a dependency
//! CYCLE. The established sim-crate battle-integration idiom (gtw317 / gtw323 / gtw341)
//! drives `setup_battle_on_request` via a `SetupBattleRequested` message against a
//! `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT
//! production wiring, `app.update()`-driven, `world_mut()` for setup / mutation / assertion
//! (bevy-traps #7's headless-test carve-out). That is what these tests use; the
//! full-state-machine `GdtfTestAppBuilder` path is exercised by the gdtf_app-level tests
//! above the sim.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, Messages, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Faction, MoveRejected, MoveRejection, MovementOccurred, Position, Speed, Stance, StanceKind,
    Tu,
    acts::MoveRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG stream.
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player.
const PLAYER: u8 = 0;

/// A deliberately SHORT view range so a cell a few tiles away is UNSEEN (non-routable) —
/// the default 14 would light the whole local field. Arbitrary test tuning, never a
/// pinned shipped magnitude.
const TEST_VIEW_RANGE: u16 = 3;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The player ganger's spawn cell.
fn player_at() -> CellLevel {
    ground(5, 5)
}

/// A cell one step EAST of the player — adjacent, open, within [`TEST_VIEW_RANGE`] (so
/// VISIBLE / routable), an affordable single open step.
fn adjacent_open() -> CellLevel {
    ground(6, 5)
}

/// A cell far beyond [`TEST_VIEW_RANGE`] — UNSEEN at spawn (the player never saw it), so
/// non-routable: any route to it must cross UNSEEN, which `find_path` refuses (GTW-353).
fn far_unseen() -> CellLevel {
    ground(40, 40)
}

/// A one-player situation: a standing player ganger (gang 0) on clear ground with the
/// given Speed (GTW-384: TU is DERIVED = `tu_base + tu_per_speed·Speed`, so a high Speed
/// yields an ample TU pool covering an open step), `player_faction` defaulting to gang 0.
/// The unaffordable test ZEROES the spawned `Tu` directly after setup (the headless-test
/// world-mutation carve-out) rather than authoring an attribute that derives to 0.
fn one_player_situation(speed: f32) -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([GangerSpawnBuilder::new()
            .at(player_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .speed(Speed::new(speed))
            .build()])
        .build_with_gangs()
}

/// Build the FULL live-runtime harness: `MinimalPlugins` + `AssetPlugin` + `ScenePlugin`
/// (the `bsn!` ganger spawn infrastructure) + the production `BattleSimPlugin` (which
/// installs the `BattleInProgress` gate on the `Simulate` band and wires the GTW-354
/// constrained move dispatch after the `occupancy_sync` chain). Seeds the persistent
/// `Load` resources a `MinimalPlugins` app has no `AssetServer` to load.
fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the
/// deferred `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    // GTW-414: insert the synthesized GangRegistry so setup_battle_on_request resolves
    // the PlacedGanger (gang, member) refs (the weapon/armor registries' precedent).
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();
    app.update();
}

/// The single player ganger entity + its current `Position`, found via a `world_mut()`
/// query (bevy-traps #7 headless-test carve-out).
fn player_entity_and_pos(app: &mut App) -> Option<(Entity, CellLevel)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    query
        .iter(world)
        .find(|(_, faction, _)| ***faction == PLAYER)
        .map(|(entity, _, position)| (entity, **position))
}

/// Drain the `MovementOccurred` log signals emitted this run.
fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

/// Drain the typed `MoveRejected` signals emitted this run.
fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

/// Clear any setup-time `MovementOccurred` / `MoveRejected` signals so a later assertion
/// sees only the commit under test (drain + discard via `len`, a `usize` with no
/// destructor — keeping clippy's `let_underscore_drop` happy).
fn clear_signals(app: &mut App) {
    let _movements: usize = drain_movements(app).len();
    let _rejects: usize = drain_rejects(app).len();
}

// === C6.1 — a teleport to a non-adjacent UNREACHABLE cell is REJECTED: no move, no
// MovementOccurred (typed Unreachable). ===

#[test]
fn unreachable_teleport_is_rejected_with_no_move() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some((actor, before)) = player_entity_and_pos(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    // Clear any setup-time signals so the assertion sees only this commit.
    clear_signals(&mut app);

    // Commit a teleport to a far cell the player has never seen (UNSEEN → non-routable).
    app.world_mut()
        .write_message(MoveRequested::new(actor, far_unseen()));
    app.update();
    app.update();

    // A typed Unreachable reject was emitted for this actor.
    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Unreachable),
        "an unreachable teleport must emit MoveRejected(Unreachable): {rejects:?}",
    );
    // NO MovementOccurred (the move was not announced).
    assert!(
        drain_movements(&mut app).is_empty(),
        "an unreachable teleport announces NO MovementOccurred",
    );
    // The actor did NOT move — the any-empty-cell teleport is dead.
    let Some((_, after)) = player_entity_and_pos(&mut app) else {
        unreachable!("the player ganger persists");
    };
    assert_eq!(
        after, before,
        "an unreachable teleport leaves the mover exactly where it was (no teleport)",
    );
}

// === C6.2 — a REACHABLE route is ACCEPTED: move + MovementOccurred, no MoveRejected. ===

#[test]
fn reachable_route_is_accepted_with_move_and_log() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some((actor, before)) = player_entity_and_pos(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    assert_eq!(
        before,
        player_at(),
        "precondition: the player starts at its spawn cell",
    );
    clear_signals(&mut app);

    // Commit a move to the adjacent, in-sight, affordable cell.
    let dest = adjacent_open();
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();
    app.update();

    // Exactly one MovementOccurred announces the step, carrying this actor.
    let movements = drain_movements(&mut app);
    assert!(
        movements.iter().any(|m| m.actor == actor),
        "a reachable affordable move must announce a MovementOccurred for the actor: {movements:?}",
    );
    // NO reject was emitted.
    assert!(
        drain_rejects(&mut app).is_empty(),
        "a reachable affordable move must emit NO MoveRejected",
    );
    // The actor actually stepped to the destination.
    let Some((_, after)) = player_entity_and_pos(&mut app) else {
        unreachable!("the player ganger persists");
    };
    assert_eq!(
        after, dest,
        "a reachable affordable move must step the mover to the destination",
    );
}

// === C6.3 — an UNAFFORDABLE route is REJECTED with NO partial move (the mover stays
// put). ===

#[test]
fn unaffordable_route_is_rejected_with_no_partial_move() {
    let mut app = battle_app();
    // Spawn with an ample derived TU pool, then ZERO it directly (GTW-384: TU is now a
    // derived pool — the headless-test world-mutation carve-out, bevy-traps #7a). The
    // mover can then afford NO positive-cost route — but the destination is still reachable
    // (adjacent, in-sight), so the rejection is Unaffordable, not Unreachable.
    drive_setup(&mut app, one_player_situation(20.0));

    let Some((actor, before)) = player_entity_and_pos(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    // Drain the mover's TU to zero so any positive-cost route is unaffordable.
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(actor) {
        *tu = Tu::new(0);
    }
    assert_eq!(
        before,
        player_at(),
        "precondition: the player starts at its spawn cell",
    );
    clear_signals(&mut app);

    // Commit a move to the reachable adjacent cell the mover cannot afford.
    let dest = adjacent_open();
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();
    app.update();

    // A typed Unaffordable reject was emitted (NOT Unreachable — the cell IS reachable).
    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Unaffordable),
        "an unaffordable route must emit MoveRejected(Unaffordable): {rejects:?}",
    );
    // NO MovementOccurred (no partial step was announced).
    assert!(
        drain_movements(&mut app).is_empty(),
        "an unaffordable route announces NO MovementOccurred (no partial move)",
    );
    // The mover stayed exactly put — NO partial move.
    let Some((_, after)) = player_entity_and_pos(&mut app) else {
        unreachable!("the player ganger persists");
    };
    assert_eq!(
        after, before,
        "an unaffordable route leaves the mover exactly where it was (NO partial move)",
    );
}
