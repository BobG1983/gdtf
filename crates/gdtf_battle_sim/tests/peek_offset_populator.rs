//! GTW-406 — the automatic positional [`PeekOffset`] populator, driven END-TO-END on
//! the REAL [`sync_peek_offsets`] system and the REAL
//! [`has_los`](gdtf_battle_sim::los::has_los) / [`has_los_peeking`](gdtf_battle_sim::los::has_los_peeking)
//! consumer.
//!
//! Proves the producer half of the GTW-393 wall-peek path:
//!
//! 1. **A ganger at a wall corner is AUTOMATICALLY given a peek toward the open edge**,
//!    and that populated value CAUSES around-corner sight a centred ganger CANNOT get
//!    (`has_los` centred → blocked, `has_los_peeking` with the populated offset → sighted)
//!    — a positive producer→consumer bridge, not merely "the offset is non-zero".
//! 2. **Moving away from the corner CLEARS the peek** (no stale around-corner peek).
//! 3. **Open ground yields NO peek** (the offset stays `default`).
//! 4. **A `CoverDestroyed` re-evaluates a stationary ganger's peek and clears it** — proving
//!    the cover trigger, the stationary-ganger full-scan path, AND the run-condition's
//!    unconditional-drain fix (the ganger never moves, so only the cover signal fires it).
//!
//! HARNESS NOTE — a wall, in the live sim, lives in BOTH grids: the `OccupancyGrid` as
//! [`TerrainKind::Wall`] (what `corner_lean`'s `is_blocked` reads) AND the `CoverLedger`
//! as a HIGH [`CoverEntry`] (what the LOS march actually stops on — `impact_at` blocks on
//! occupants / cover / slabs, never bare terrain). `setup_battle` pours every wall into
//! both. These tests do the same: `set_terrain(Wall/Cover)` drives corner detection and a
//! co-located HIGH `CoverEntry` drives the march block, exactly mirroring the runtime.
//!
//! The producer is a focused `MinimalPlugins` app with just the populator system + its
//! `CoverDestroyed` buffer (the sim-crate headless idiom — `gdtf_test_utils` would be a
//! dep cycle, see `squad_fog_recompute`). The consumer assertions call the pure `has_los*` functions
//! directly with the populated `PeekOffset` read off the entity.

use bevy::{
    app::App,
    math::Vec2,
    prelude::{IntoScheduleConfigs, MinimalPlugins},
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Facing,
    los::{Observer, PeekOffset, Target, has_los, has_los_peeking},
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, StairEyeOffset, TerrainKind},
    occupancy_sync::{CoverDestroyed, sync_destroyed_cover},
    peek_sync::{peek_population_needed, sync_peek_offsets},
    prelude::{Direction, Faction, Position, Stance, StanceKind},
    surface::SurfaceGrid,
    test_support::GangerEntityBuilder,
    tuning::CombatTuning,
};

/// A ground-floor `(cell, level)` key.
fn key(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The ganger's corner-hugging spawn cell.
fn ganger_cell() -> CellLevel {
    key(3, 3)
}

/// The expected automatic lean at [`ganger_cell`] for the corner the fixtures build:
/// a blocker East (4,3) whose NORTH end (4,4) is blocked and SOUTH end (4,2) open →
/// lean South `(0, -0.4)`.
const fn expected_lean() -> PeekOffset {
    PeekOffset::new(Vec2::new(0.0, -0.4))
}

/// A grid with a corner blocker East of [`ganger_cell`]: `(4,3)` + `(4,4)` set to
/// `terrain`, `(4,2)` left open. With `terrain = Wall` (or `Cover`), `is_blocked` is
/// `true` at the two blocker cells, so `corner_lean` reads the corner.
fn corner_grid(terrain: TerrainKind) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    grid.set_terrain(key(4, 3), terrain);
    grid.set_terrain(key(4, 4), terrain);
    grid
}

/// The `CoverLedger` the LOS march stops on: a HIGH `CoverEntry` at each blocker cell
/// `(4,3)` + `(4,4)` (the runtime co-locates a wall's ledger entry with its terrain).
fn corner_cover() -> CoverLedger {
    let mut cover = CoverLedger::new();
    for at in [key(4, 3), key(4, 4)] {
        cover.insert(
            at,
            CoverEntry::seeded(
                CoverHp::new(50),
                HeightBand::High,
                ArmorProtection::new(5),
                ArmorHardness::new(2),
            ),
        );
    }
    cover
}

/// A focused headless app: `MinimalPlugins`, the `CoverDestroyed` buffer (the
/// run-condition's `MessageReader` param), the `grid` resource, and the populator system
/// gated on its real run-condition.
fn populator_app(grid: OccupancyGrid) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<CoverDestroyed>();
    app.insert_resource(grid);
    app.add_systems(
        bevy::prelude::Update,
        sync_peek_offsets.run_if(peek_population_needed),
    );
    app
}

/// Spawn a ganger with the five components the populator + the LOS bridge need; return
/// its `Entity` handle.
fn peeking_ganger(app: &mut App, at: CellLevel) -> bevy::prelude::Entity {
    let ganger = GangerEntityBuilder::new()
        .at(at)
        .stance(StanceKind::Standing)
        .facing(Direction::North)
        .faction(Faction::new(0))
        .spawn(app.world_mut());
    // The suite-specific peek substrate (not a common ganger knob) rides a direct insert.
    app.world_mut()
        .entity_mut(ganger)
        .insert(PeekOffset::default());
    ganger
}

/// The entity's current `PeekOffset` (copied), if present.
fn peek_of(app: &App, entity: bevy::prelude::Entity) -> Option<PeekOffset> {
    app.world().get::<PeekOffset>(entity).copied()
}

/// Move a ganger to `to` (a `Position` mutation in the test body — bevy-traps #7
/// carve-out, the analogue of an accepted walk step).
fn move_ganger(app: &mut App, entity: bevy::prelude::Entity, to: CellLevel) {
    if let Some(mut position) = app.world_mut().get_mut::<Position>(entity) {
        *position = Position::new(to);
    }
}

// === AC1 — automatic peek toward the open edge AND the positive producer→consumer LOS
// bridge (centred blocked, peeking sighted). ===

#[test]
fn populates_corner_peek_and_enables_around_corner_los() {
    let mut app = populator_app(corner_grid(TerrainKind::Wall));
    let entity = peeking_ganger(&mut app, ganger_cell());

    // First update: the freshly-spawned Position reads Changed (Bevy first-run), so
    // peek_population_needed fires and sync_peek_offsets derives the corner peek.
    app.update();

    let Some(peek) = peek_of(&app, entity) else {
        unreachable!("the spawned ganger carries PeekOffset");
    };
    assert_eq!(
        peek,
        expected_lean(),
        "a ganger at the E-wall corner (open South end) must be auto-leaned South (0, -0.4)",
    );

    // --- The positive bridge: the populated peek CAUSES sight a centred eye cannot get.
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = corner_cover();
    // The same corner geometry the populator read (the populator never mutates the grid).
    let occupancy = corner_grid(TerrainKind::Wall);

    let obs_pos = Position::new(ganger_cell());
    let obs_stance = Stance::new(StanceKind::Standing);
    let obs_facing = Facing::new(Direction::North);
    // A target around the open (South) end of the corner — (5,2,0): the centred ray from
    // (3.5,3.5) crosses the HIGH cover cell (4,3); the South-leaned ray skirts it via the
    // open (4,2).
    let tgt_pos = Position::new(key(5, 2));
    let tgt_stance = Stance::new(StanceKind::Standing);
    let target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };
    let centred = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };

    let centred_sighted = has_los(
        &centred,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        |_| false,
    );
    assert!(
        !*centred_sighted,
        "the CENTRED eye must be BLOCKED by the HIGH corner cover (the control)",
    );

    let peeked_sighted = has_los_peeking(
        &centred,
        &target,
        peek, // the AUTOMATICALLY populated offset — not a hand-set value
        &occupancy,
        &surface,
        &cover,
        &tuning,
        |_| false,
    );
    assert!(
        *peeked_sighted,
        "the AUTO-POPULATED peek must let the ganger SEE the target around the corner",
    );
    assert_ne!(
        *centred_sighted, *peeked_sighted,
        "the populated peek must CHANGE the verdict (the producer→consumer bridge)",
    );
}

// === AC2 — moving away from the corner CLEARS the peek (no stale around-corner peek). ===

#[test]
fn moving_away_clears_the_peek() {
    let mut app = populator_app(corner_grid(TerrainKind::Wall));
    let entity = peeking_ganger(&mut app, ganger_cell());
    app.update();
    // Precondition: the corner peek was populated.
    assert_eq!(
        peek_of(&app, entity),
        Some(expected_lean()),
        "precondition: the corner ganger has the South lean",
    );

    // Move to open ground far from any corner — trips Changed<Position>, re-runs the scan.
    move_ganger(&mut app, entity, key(10, 10));
    app.update();

    assert_eq!(
        peek_of(&app, entity),
        Some(PeekOffset::default()),
        "after moving to open ground the peek must CLEAR to default (no stale peek)",
    );
}

// === AC3 — a ganger on open ground gets NO peek. ===

#[test]
fn open_ground_gets_no_peek() {
    let mut app = populator_app(corner_grid(TerrainKind::Wall));
    // Spawn far from the corner — open ground in every cardinal.
    let entity = peeking_ganger(&mut app, key(20, 20));
    app.update();

    assert_eq!(
        peek_of(&app, entity),
        Some(PeekOffset::default()),
        "a ganger on open ground hugs no corner → PeekOffset stays default",
    );
}

// === AC4 — a CoverDestroyed re-evaluates a STATIONARY ganger's peek and clears it. ===

#[test]
fn cover_destroyed_clears_a_stationary_peek() {
    // The corner is COVER (so it can be destroyed); add sync_destroyed_cover BEFORE the
    // populator so the destroyed-cover set is current when corner_lean reads is_blocked.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<CoverDestroyed>();
    app.insert_resource(corner_grid(TerrainKind::Cover));
    app.add_systems(
        bevy::prelude::Update,
        (
            sync_destroyed_cover,
            sync_peek_offsets.run_if(peek_population_needed),
        )
            .chain(),
    );
    let entity = peeking_ganger(&mut app, ganger_cell());
    app.update();
    assert_eq!(
        peek_of(&app, entity),
        Some(expected_lean()),
        "precondition: the cover-corner ganger has the South lean",
    );

    // Smash the East cover cell — the ganger does NOT move. Only the CoverDestroyed signal
    // fires the populator (proving the cover trigger + the unconditional-drain fix), and
    // the now-open corner clears the peek.
    app.world_mut()
        .write_message(CoverDestroyed::new(key(4, 3)));
    app.update();

    assert_eq!(
        peek_of(&app, entity),
        Some(PeekOffset::default()),
        "destroying the corner cover must re-evaluate the STATIONARY ganger and clear its peek",
    );
}
