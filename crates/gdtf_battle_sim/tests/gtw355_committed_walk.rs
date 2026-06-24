//! GTW-355 (E7 · GTW-12g) — the COMMITTED step-by-step walk: an accepted move no longer
//! jumps to the destination; it walks the planned route ONE cell per tick, charging each
//! step atomically, bump-stopping on a live obstacle, and halting the moment an enemy is
//! revealed or a reaction shot interrupts. Proven end-to-end on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path.
//!
//! The four C7 behaviors:
//!
//! - **(a) BUMP-STOP** — a walk whose next cell becomes OCCUPIED after the route was
//!   planned stops at the last free step, charged ONLY the steps actually taken (no
//!   teleport, no co-location).
//! - **(b) §48 BIT-IDENTITY** — an UNINTERRUPTED walk's total charged TU equals the
//!   `find_path` `Path::total()` for that route, bit-for-bit (a wrong per-step cost source
//!   would break this — pin-discriminating).
//! - **(c) STOP-ON-REVEAL** — a walk that brings a previously-UNSEEN enemy into the squad
//!   VISIBLE set after a step halts immediately, charged only the steps taken.
//! - **(d) STOP-ON-INTERRUPT** — a SYNTHETIC `ReactionShotFired` mid-walk halts the walk,
//!   charged only the steps taken. (The reaction-fire PRODUCER is GTW-38-future; this slice
//!   builds the receiving hook only.)
//!
//! RELATIONS-ONLY, pin-discriminating, NO magnitude pins: TU is compared by relations
//! (`spent == 0` / `spent > 0` / `walk-TU == find_path total`), never against a shipped
//! move-cost magnitude.
//!
//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`", as in GTW-354):
//! the sim crate is the LOW crate — `gdtf_test_utils` depends on `gdtf_app` which depends
//! on `gdtf_battle_sim`, so a sim-crate dev-dep on `gdtf_test_utils` would be a dependency
//! CYCLE. The established sim-crate battle-integration idiom (gtw341 / gtw354) drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Faction, Position, ReactionShotFired, Speed, SquadVisibility, Stance, StanceKind, Tu,
    WalkInProgress,
    acts::MoveRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    pathfinder::{PlanningView, find_path},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    vertical::VerticalLinkGraph,
    visibility::FactionRelation,
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG stream.
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player.
const PLAYER: u8 = 0;
/// Gang `1` is the enemy.
const ENEMY: u8 = 1;

/// A view range that LIGHTS the local field a few cells out (so a short walk routes), yet
/// is short enough that a cell several tiles away is UNSEEN at spawn. Arbitrary test
/// tuning, never a pinned shipped magnitude.
const TEST_VIEW_RANGE: u16 = 4;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The player ganger's spawn cell.
fn player_at() -> CellLevel {
    ground(5, 5)
}

/// Build the FULL live-runtime harness (the GTW-354 `battle_app` idiom): `MinimalPlugins`,
/// `AssetPlugin`, `ScenePlugin`, and `BattleSimPlugin`, with the persistent `Load`
/// resources a `MinimalPlugins` app has no `AssetServer` to load, and a short view range.
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
fn drive_setup(app: &mut App, situation: Situation) {
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();
    app.update();
}

/// The player ganger entity (gang 0), found via a `world_mut()` query.
fn player_entity(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, faction)| ***faction == PLAYER)
        .map(|(entity, _)| entity)
}

/// The current `(Position, Tu)` of `entity`.
fn pos_and_tu(app: &App, entity: Entity) -> Option<(CellLevel, u8)> {
    let position = app.world().get::<Position>(entity).map(|p| **p)?;
    let tu = app.world().get::<Tu>(entity).map(|t| **t)?;
    Some((position, tu))
}

/// Whether `entity` still carries a `WalkInProgress` (the walk is in flight).
fn is_walking(app: &App, entity: Entity) -> bool {
    app.world().get::<WalkInProgress>(entity).is_some()
}

/// The `find_path` total TU for the route from `start` to `goal`, computed over the LIVE
/// world resources through the EXACT production planning path (the squad fog + a player-
/// relative occupant resolver) — so it is the same total `dispatch_move` gated on.
fn plan_total(app: &App, start: CellLevel, goal: CellLevel) -> Option<u8> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let links = app.world().get_resource::<VerticalLinkGraph>()?;
    let squad = app.world().get_resource::<SquadVisibility>()?;
    let tuning = app.world().get_resource::<CombatTuning>()?;
    // The player squad sees its own gang; any non-player occupant is Other (the
    // dispatch_move `relation_to` shape, player-relative).
    let planning = PlanningView::new(squad, |_occupant| FactionRelation::Other);
    let path = find_path(start, goal, grid, links, tuning, &planning).ok()?;
    Some(*path.total())
}

/// Drive `app.update()` to dispatch a freshly-written `MoveRequested` and then walk it to
/// completion — at least one tick (so the accept starts the walk) and then until the
/// `WalkInProgress` is gone (the walk finished or stopped) or a generous tick budget is
/// exhausted (no open-ended loop). The first tick both dispatches the request AND takes
/// the first step (`advance_walk` runs `.after(dispatch_move)` in the same band).
fn run_until_walk_ends(app: &mut App, entity: Entity) {
    for _ in 0..32 {
        app.update();
        if !is_walking(app, entity) {
            return;
        }
    }
}

/// A one-player situation: a standing player ganger (gang 0) at [`player_at`] with the
/// given Speed (GTW-384: the derived TU budget = `tu_base + tu_per_speed·Speed`, so a
/// high Speed gives an ample TU pool — the absolute starting TU is read from the world,
/// the walk tests assert TU SPENT relative to it, never a pinned start).
fn one_player_situation(speed: f32) -> Situation {
    SituationBuilder::new()
        .with_gangers([GangerSpawnBuilder::new()
            .at(player_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .speed(Speed::new(speed))
            .build()])
        .build()
}

/// A player + a far enemy situation: the player at [`player_at`], an enemy (gang 1) at
/// `enemy_cell` (placed beyond [`TEST_VIEW_RANGE`] so it is UNSEEN at spawn). Both get
/// the given Speed (→ an ample derived TU pool, GTW-384).
fn player_and_enemy_situation(speed: f32, enemy_cell: CellLevel) -> Situation {
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(speed))
                .build(),
            GangerSpawnBuilder::new()
                .at(enemy_cell)
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(speed))
                .build(),
        ])
        .build()
}

// === C7(b) — an UNINTERRUPTED multi-step walk charges EXACTLY the find_path total
// (the §48 bit-identity, end-to-end). ===

#[test]
fn uninterrupted_walk_charges_exactly_the_find_path_total() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // A multi-cell destination a few tiles east, within view range (so routable) and
    // affordable with the spawn TU.
    let dest = ground(8, 5);
    let Some(expected_total) = plan_total(&app, start, dest) else {
        unreachable!("an open in-sight multi-step route exists");
    };
    assert!(
        expected_total > 0,
        "precondition: a real multi-step route has a positive total",
    );

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    run_until_walk_ends(&mut app, actor);

    let Some((after, tu_after)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_eq!(
        after, dest,
        "an uninterrupted walk reaches the destination cell",
    );
    let spent = tu_before.saturating_sub(tu_after);
    assert_eq!(
        spent, expected_total,
        "the stepped walk charges EXACTLY the find_path total, bit-for-bit (§48)",
    );
}

// === C7(a) — a walk whose next cell is OCCUPIED after planning BUMP-STOPS at the last
// free step, charged only the steps taken. ===

#[test]
fn walk_bump_stops_when_next_cell_becomes_occupied() {
    let mut app = battle_app();
    // Two players so we have a second body to drop into the route mid-walk; both gang 0
    // so neither blocks the other's route by visibility (own-squad), but an OCCUPANT slot
    // always bump-stops regardless of faction.
    let situation = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(20.0))
                .build(),
            GangerSpawnBuilder::new()
                .at(ground(20, 20))
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(20.0))
                .build(),
        ])
        .build();
    drive_setup(&mut app, situation);

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // A 3-cell east walk: (5,5) -> (6,5) -> (7,5) -> (8,5).
    let dest = ground(8, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    // One tick: the accept starts the walk and takes the FIRST step to (6,5).
    app.update();
    let Some((after_first, tu_after_first)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        after_first, start,
        "precondition: the walk took its first step",
    );
    assert!(
        is_walking(&app, actor),
        "precondition: the walk is still in flight (more cells ahead)",
    );

    // Now drop an OBSTACLE onto the NEXT cell ahead (7,5) — occupied AFTER the route was
    // planned (occupancy_sync re-runs each tick, so a fresh occupant appears mid-walk).
    let blocker_cell = ground(7, 5);
    let blocker = app.world_mut().spawn_empty().id();
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(blocker_cell, Some(blocker));
    }

    // Run the walk to its end: it must BUMP-STOP before entering (7,5).
    run_until_walk_ends(&mut app, actor);

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        final_cell, blocker_cell,
        "the mover NEVER enters the now-occupied cell (no co-location)",
    );
    assert_ne!(
        final_cell, dest,
        "the bump-stop halts the walk SHORT of the destination",
    );
    // Charged only the steps actually taken: a strictly positive spend (it moved) that is
    // STRICTLY LESS than the full planned route would have cost (it stopped short).
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the full route was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "a bump-stopped walk charges only the ground covered (steps taken), not the full \
         route: spent={spent}, full={full_total}",
    );
    // And it did move at least one cell (the first step landed before the obstacle).
    assert!(
        tu_after_first < tu_before,
        "the first step charged its cost",
    );
}

// === C7(c) — a walk that REVEALS a previously-UNSEEN enemy halts immediately, charged
// only the steps taken. ===

#[test]
fn walk_stops_when_a_new_enemy_is_revealed() {
    let mut app = battle_app();
    // An enemy parked east at (11,5), beyond TEST_VIEW_RANGE from the player spawn
    // (Chebyshev 6 > 4), so it is UNSEEN at walk start.
    let enemy_cell = ground(11, 5);
    drive_setup(&mut app, player_and_enemy_situation(20.0, enemy_cell));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // Walk east to (9,5) — within view range at spawn (Chebyshev 4, so routable) and short
    // of the enemy, but the approach brings the enemy into view (Chebyshev to (11,5)
    // shrinks below TEST_VIEW_RANGE mid-walk), revealing it.
    let dest = ground(9, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    run_until_walk_ends(&mut app, actor);

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    // The reveal halts the walk SHORT of the destination (it stops the tick the enemy
    // enters the squad VISIBLE set).
    assert_ne!(
        final_cell, dest,
        "a revealed enemy halts the walk before the destination (the §44 ambush)",
    );
    assert_ne!(
        final_cell, start,
        "the walk took at least one step before the reveal",
    );
    // Charged only the steps taken (a positive, partial spend).
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the route to dest was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "a reveal-stopped walk charges only the ground covered: spent={spent}, full={full_total}",
    );
    // The walk is no longer in flight (it was halted, not merely paused).
    assert!(
        !is_walking(&app, actor),
        "a revealed enemy removes the WalkInProgress (the walk halted)",
    );
    // POSITIVE: the enemy IS now in the squad VISIBLE set — the reveal mechanic (the
    // recompute the step's Position write triggered) is what halted the walk, not a
    // coincidental obstacle (the open route had none short of the enemy).
    let enemy_now_visible = app
        .world()
        .get_resource::<SquadVisibility>()
        .is_some_and(|squad| squad.is_cell_visible(&enemy_cell));
    assert!(
        enemy_now_visible,
        "the enemy entered the squad VISIBLE set — the reveal is what stopped the walk",
    );
}

// === C7(d) — a SYNTHETIC reaction-shot interrupt mid-walk halts the walk, charged only
// the steps taken. (The PRODUCER is GTW-38-future.) ===

#[test]
fn walk_stops_on_a_synthetic_reaction_interrupt() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some(actor) = player_entity(&mut app) else {
        unreachable!("setup spawns the player ganger");
    };
    let Some((start, tu_before)) = pos_and_tu(&app, actor) else {
        unreachable!("the player has a Position and Tu");
    };

    // Start a multi-step walk east.
    let dest = ground(9, 5);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    // One tick: the walk starts and takes its first step.
    app.update();
    let Some((after_first, _)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert_ne!(
        after_first, start,
        "precondition: the walk took a first step"
    );
    assert!(
        is_walking(&app, actor),
        "precondition: the walk is still in flight",
    );

    // SYNTHETIC emit of the reaction-shot interrupt aimed at the mover (the GTW-38-future
    // producer would emit this; here the test plays the producer's role).
    app.world_mut().write_message(ReactionShotFired::new(actor));
    // The next advance_walk tick reads the interrupt and halts the walk.
    app.update();

    let Some((final_cell, tu_final)) = pos_and_tu(&app, actor) else {
        unreachable!("the player persists");
    };
    assert!(
        !is_walking(&app, actor),
        "a reaction-shot interrupt halts the walk (removes the WalkInProgress)",
    );
    assert_ne!(
        final_cell, dest,
        "the interrupt halts the walk before the destination",
    );
    // Charged only the steps taken before the interrupt (a positive, partial spend; the
    // interrupt itself charges nothing).
    let Some(full_total) = plan_total(&app, start, dest) else {
        unreachable!("the route to dest was plannable at start");
    };
    let spent = tu_before.saturating_sub(tu_final);
    assert!(
        spent > 0 && spent < full_total,
        "an interrupt-stopped walk charges only the ground covered: spent={spent}, \
         full={full_total}",
    );
}
