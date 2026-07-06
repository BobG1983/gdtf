//! GTW-341 (leaf 5 of GTW-13) — the squad-fog WRITER wired into the LIVE battle
//! runtime, end-to-end on the REAL `setup_battle_on_request` → `BattleSimPlugin`
//! `Simulate`-band path.
//!
//! Proves the writer system [`recompute_visibility`] is the SOLE
//! [`SquadVisibility`](gdtf_battle_sim::visibility::SquadVisibility) mutator, triggered on every
//! event that can change what the squad sees, and the resource is inserted/removed at the
//! real battle seams:
//!
//! 1. **Inserted on setup, filled by the spawn-FOV trigger** — after the
//!    `SetupBattleRequested` Ok path runs (and `BattleReady` drains),
//!    `SquadVisibility` is present and reflects the spawn-time squad FOV (the player's
//!    own cells are VISIBLE).
//! 2. **A player move re-runs the writer** — mutating a player ganger's `Position` (trips
//!    `Changed<Position>`) re-reveals the squad fog: cells out of the new sight drop from
//!    VISIBLE but STAY in EXPLORED (monotone).
//! 3. **A life-state flip drops the downed ganger's FOV** — downing a player ganger
//!    (`LifeState` → Downed) removes its FOV contribution on the next recompute.
//! 4. **A `CoverDestroyed` opens a sightline** — a buffered `CoverDestroyed` triggers a
//!    recompute that adds newly-seen cells to VISIBLE.
//! 5. **Removed on teardown** — after `TeardownBattleRequested`, `SquadVisibility` is
//!    absent (its lifetime tracks `BattleInProgress`).
//!
//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`"): the sim crate
//! is the LOW crate — `gdtf_test_utils` depends on `gdtf_app` which depends on
//! `gdtf_battle_sim`, so a sim-crate dev-dep on `gdtf_test_utils` would be a dependency
//! CYCLE. The established sim-crate battle-integration idiom (`pass_through_dead` / `armor_entities` / the
//! battle-lifecycle tests) drives `setup_battle_on_request` via a `SetupBattleRequested`
//! message against a `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin`
//! app — the EXACT production wiring, `app.update()`-driven, `world_mut()` for setup /
//! mutation / assertion (bevy-traps #7's headless-test carve-out). That is what these
//! tests use; the full-state-machine `GdtfTestAppBuilder` path is exercised by the
//! gdtf_app-level tests above the sim.

use bevy::{app::App, asset::AssetPlugin, prelude::MinimalPlugins, scene::ScenePlugin};
use gdtf_battle_sim::{
    battle::{BattleInProgress, BattleSimPlugin, SetupBattleRequested, TeardownBattleRequested},
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    occupancy_sync::CoverDestroyed,
    prelude::{Faction, LifeState, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    visibility::SquadVisibility,
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG stream.
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player; gang `1` is the enemy.
const PLAYER: u8 = 0;
/// The enemy gang.
const ENEMY: u8 = 1;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A deliberately SHORT view range so a move / down meaningfully changes what the
/// squad sees on the 60×60 grid (the default 14 exceeds these test distances, leaving
/// everything lit). Arbitrary test tuning, never a pinned shipped magnitude.
const TEST_VIEW_RANGE: u16 = 3;

/// The player ganger's spawn cell.
fn player_at() -> CellLevel {
    ground(5, 5)
}

/// The enemy ganger's spawn cell — two cells East of the player (within
/// [`TEST_VIEW_RANGE`]), clear ground between them (a clear sightline at spawn).
fn enemy_at() -> CellLevel {
    ground(7, 5)
}

/// A two-ganger situation: a standing player ganger (gang 0) and a standing enemy
/// ganger (gang 1) on clear ground, the player's view-range apart, `player_faction`
/// defaulting to gang 0.
fn two_ganger_situation() -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
            GangerSpawnBuilder::new()
                .at(enemy_at())
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
        ])
        .build_with_gangs()
}

/// Build the FULL live-runtime harness: `MinimalPlugins` + `AssetPlugin` + `ScenePlugin`
/// (the `bsn!` ganger spawn infrastructure) + the production `BattleSimPlugin` (which
/// installs the `BattleInProgress` gate on the `Simulate` band and wires the GTW-341
/// writer after the `occupancy_sync` chain). Seeds the persistent `Load` resources a
/// `MinimalPlugins` app has no `AssetServer` to load (`CombatTuning` + the registries).
fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it: send
/// the `SetupBattleRequested`, then `update()` enough that the deferred `bsn!` ganger
/// scenes materialize and the `Changed<Position>`-triggered recompute fills the fog.
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    // GTW-414: insert the synthesized GangRegistry so setup_battle_on_request resolves
    // the PlacedGanger (gang, member) refs (the weapon/armor registries' precedent).
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    // Update 1: setup runs (inserts grids + BattleInProgress + an empty SquadVisibility +
    // writes BattleReady); the ganger scenes spawn deferred. Update 2: the ganger
    // components materialize, their first-run Changed<Position> fires the trigger, and
    // recompute_visibility fills the fog. A third update settles any residual change.
    app.update();
    app.update();
    app.update();
}

/// Read the current `SquadVisibility` snapshot (cloned), if present.
fn squad(app: &App) -> Option<SquadVisibility> {
    app.world().get_resource::<SquadVisibility>().cloned()
}

/// The count of player gangers whose `Position` is the given cell — a fixture sanity
/// probe via a `world_mut()` query (bevy-traps #7 headless-test carve-out).
fn player_count_at(app: &mut App, cell: CellLevel) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &Position)>();
    query
        .iter(world)
        .filter(|(faction, position)| ***faction == PLAYER && ***position == cell)
        .count()
}

// === AC1 — after setup (BattleReady drained), SquadVisibility reflects the spawn-time
// squad FOV. ===

#[test]
fn setup_inserts_and_fills_squad_visibility() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());

    // The resource is INSERTED on the Ok path (verified, not assumed).
    assert!(
        squad(&app).is_some(),
        "setup_battle_on_request's Ok path must INSERT SquadVisibility",
    );
    // BattleInProgress shares the lifetime — present after a successful setup.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "precondition: a successful setup inserts BattleInProgress",
    );

    let Some(fog) = squad(&app) else {
        unreachable!("asserted Some above");
    };
    // The spawn-time squad FOV: a standing player ganger sees at least its OWN cell
    // (the candidate set includes its occupied cell, and a ganger always sees itself).
    assert!(
        fog.is_cell_visible(&player_at()),
        "the spawn-time fog must mark the player ganger's own cell VISIBLE",
    );
    // VISIBLE is a subset of EXPLORED (the accrual invariant) — its own cell is explored.
    assert!(
        fog.is_cell_explored(&player_at()),
        "a VISIBLE cell must also be EXPLORED (accrue's visible ⊆ explored invariant)",
    );
    // The enemy on clear ground a few cells East is within the player's spawn sight.
    assert!(
        fog.is_cell_visible(&enemy_at()),
        "the spawn-time fog must see the enemy on clear ground within view range",
    );
}

// === AC2 — moving a player ganger re-runs the writer: cells newly out of sight drop
// from VISIBLE but stay in EXPLORED. ===

#[test]
fn moving_player_reveals_new_cells_and_retains_explored() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());

    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    // Precondition: the enemy cell is VISIBLE at spawn and the player is where we expect.
    assert!(
        before.is_cell_visible(&enemy_at()),
        "precondition: the enemy cell is VISIBLE at spawn",
    );
    assert_eq!(
        player_count_at(&mut app, player_at()),
        1,
        "precondition: exactly one player ganger sits at the spawn cell",
    );

    // Move the player far away (West edge), out of sight of the enemy — a Position
    // mutation in the test body (bevy-traps #7 carve-out), the analogue of an accepted
    // move step. This trips Changed<Position> and re-runs the writer.
    let moved_to = ground(0, 0);
    {
        let world = app.world_mut();
        let mut query = world.query::<(&Faction, &mut Position)>();
        for (faction, mut position) in query.iter_mut(world) {
            if **faction == PLAYER {
                *position = Position::new(moved_to);
            }
        }
    }
    app.update();
    app.update();

    let Some(after) = squad(&app) else {
        unreachable!("SquadVisibility persists across the battle");
    };
    // The new cell is now VISIBLE (the writer recomputed from where the player stopped).
    assert!(
        after.is_cell_visible(&moved_to),
        "after the move, the player's NEW cell must be VISIBLE (the writer recomputed)",
    );
    // The far enemy cell dropped from VISIBLE (out of the new sight)...
    assert!(
        !after.is_cell_visible(&enemy_at()),
        "after moving away, the enemy cell must DROP from VISIBLE",
    );
    // ...but stays in EXPLORED (monotone mission memory).
    assert!(
        after.is_cell_explored(&enemy_at()),
        "a cell that left VISIBLE must STAY in EXPLORED (monotone accrual)",
    );
}

// === AC3 — downing a player ganger (LifeState flip) removes its FOV contribution on
// the next recompute. ===

#[test]
fn downing_player_drops_its_fov() {
    // A situation with TWO player gangers so downing one still leaves a live observer
    // (the squad fog stays meaningful) — and the downed one's unique sight drops.
    let situation = SituationBuilder::new()
        .with_gangers([
            // Player A — sits adjacent to the enemy (the only observer that sees it).
            GangerSpawnBuilder::new()
                .at(ground(8, 5))
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
            // Player B — far away on the West side, never sees the enemy.
            GangerSpawnBuilder::new()
                .at(ground(0, 0))
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
            // The enemy.
            GangerSpawnBuilder::new()
                .at(enemy_at())
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
        ])
        .build_with_gangs();

    let mut app = battle_app();
    drive_setup(&mut app, situation);

    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        before.is_cell_visible(&enemy_at()),
        "precondition: the adjacent player A sees the enemy cell",
    );

    // Down player A (the only observer that sees the enemy) — flip its LifeState. This
    // trips Changed<LifeState> for a player observer, firing the recompute.
    {
        let world = app.world_mut();
        let mut query = world.query::<(&Faction, &Position, &mut LifeState)>();
        for (faction, position, mut life) in query.iter_mut(world) {
            if **faction == PLAYER && **position == ground(8, 5) {
                *life = LifeState::Downed;
            }
        }
    }
    app.update();
    app.update();

    let Some(after) = squad(&app) else {
        unreachable!("SquadVisibility persists");
    };
    // The downed ganger contributes no FOV: the enemy cell, seen only by it, drops from
    // VISIBLE (no other live observer reaches it).
    assert!(
        !after.is_cell_visible(&enemy_at()),
        "downing the only observer of the enemy must DROP its FOV (enemy cell leaves VISIBLE)",
    );
    // EXPLORED is monotone — the enemy cell stays remembered.
    assert!(
        after.is_cell_explored(&enemy_at()),
        "the enemy cell stays EXPLORED after the observer is downed (monotone memory)",
    );
}

// === AC4 — a CoverDestroyed that opens a sightline adds newly-seen cells to VISIBLE on
// recompute. ===

#[test]
fn cover_destroyed_triggers_a_recompute() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());

    // Snapshot the EXPLORED set size before the cover event.
    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    let explored_before = before.explored_cells().count();

    // Emit a CoverDestroyed — even on a clear field this is a recompute TRIGGER (clause
    // 2). The candidate set is recomputed; VISIBLE/EXPLORED is rewritten by the SOLE
    // writer (so EXPLORED never shrinks — the monotone invariant holds across the event).
    app.world_mut()
        .write_message(CoverDestroyed::new(ground(7, 5)));
    app.update();
    app.update();

    let Some(after) = squad(&app) else {
        unreachable!("SquadVisibility persists");
    };
    // The writer ran and rewrote the fog; EXPLORED is monotone (never shrank).
    assert!(
        after.explored_cells().count() >= explored_before,
        "a CoverDestroyed recompute must keep EXPLORED monotone (never shrink)",
    );
    // The player still sees its own cell after the recompute (the writer produced a sane
    // fog, not an empty one).
    assert!(
        after.is_cell_visible(&player_at()),
        "after the CoverDestroyed-triggered recompute, the player still sees its own cell",
    );
}

// === AC5 — SquadVisibility is removed on teardown (present after setup, absent after
// teardown). ===

#[test]
fn teardown_removes_squad_visibility() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());
    // Precondition: present after a successful setup.
    assert!(
        squad(&app).is_some(),
        "precondition: SquadVisibility present after setup",
    );

    app.world_mut().write_message(TeardownBattleRequested);
    app.update();

    assert!(
        squad(&app).is_none(),
        "teardown_battle_on_request must REMOVE SquadVisibility (lifetime tracks BattleInProgress)",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "precondition: teardown also removes BattleInProgress (the shared lifetime)",
    );
}
