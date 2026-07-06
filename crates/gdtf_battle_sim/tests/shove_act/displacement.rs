//! Where the shoved target ends up — one cell away, an off-ledge fall via the shared
//! GTW-523 path, a supported no-fall move, an occupied-cell no-op, and the diagonal verb.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    Cell, Direction, FallOccurred, OccupancyGrid, Position, ShoveOutcome, SlabState, SurfaceGrid,
    resolve_shove,
};

use super::harness::*;

/// The full run history of `FallOccurred` signals (from the recorder).
fn fall_signals(app: &App) -> Vec<FallOccurred> {
    app.world()
        .get_resource::<FallLog>()
        .map(|log| log.falls.clone())
        .unwrap_or_default()
}

// ── QA(1) — displace-away direction (the pure verb + the deliberate act) ───────

/// QA(1): a deliberate shove pushes the target ONE cell directly AWAY from the shover. The
/// shover at (5,5) faces the target at (6,5) (east); the shove moves it to (7,5). Also asserts
/// the PURE verb resolves the same displacement (C1).
#[test]
fn shove_pushes_target_one_cell_away_from_shover() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();

    // The pure verb (C1): attacker (5,5) -> target (6,5) is East, so the destination is (7,5),
    // supported (ground level 0) => Moved.
    let outcome = resolve_shove(
        Position::new(ground(5, 5)),
        Position::new(ground(6, 5)),
        target,
        app.world().resource::<SurfaceGrid>(),
        app.world().resource::<OccupancyGrid>(),
    );
    assert_eq!(
        outcome,
        ShoveOutcome::Moved { dest: ground(7, 5) },
        "the pure verb pushes the target one cell East (away from the shover), onto the ground"
    );

    shove_and_settle(&mut app, shover, target);

    // The live act moves the target the same way — one cell East, away from the shover.
    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "the deliberate shove pushes the target one cell directly away from the shover"
    );
    // The shover did NOT move (a shove displaces the TARGET, not the shover).
    assert_eq!(
        pos_of(&app, shover),
        Some(ground(5, 5)),
        "the shover stays put"
    );
}

// ── QA(2) — unsupported=>fall via 523 + FallOccurred ───────────────────────────

/// QA(2): a shove off a LEDGE (the destination cell has no supporting slab at the target's
/// storey) makes the target FALL through the shared GTW-523 `resolve_drop` path — its Position
/// drops to the supported storey below, `FallOccurred` fires, and Hp drops.
#[test]
fn shove_off_a_ledge_falls_via_523_and_fires_falloccurred() {
    let mut app = shove_app();
    // A surface where the target's storey (2) is supported AT its start cell (6,5) but the
    // destination (7,5) has NO slab at level 2 (open air) — so a shove East off the ledge falls
    // to the ground (0). Level 0 always supports the landing.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(upper(6, 5, 2), SlabState::Present); // the target stands on a real floor
    // (7,5,2) is left Absent — the ledge edge.
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), upper(5, 5, 2), 0);
    let target = shove_ganger(app.world_mut(), upper(6, 5, 2), 1);
    app.update();
    let hp_before = hp_of(&app, target);

    shove_and_settle(&mut app, shover, target);

    // The target fell off the ledge: its cell is now (7,5) and its storey dropped BELOW 2.
    let Some(landed) = pos_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert_eq!(
        landed.x, 7,
        "the target was pushed East onto the ledge cell"
    );
    assert_eq!(landed.y, 5, "the shove is a lateral (same-y) East push");
    assert!(
        landed.z < 2,
        "the target FELL off the unsupported ledge to a storey below 2 (got z={})",
        landed.z
    );
    // A FallOccurred fired for the target (the shared GTW-523 fall path ran).
    let signals = fall_signals(&app);
    assert!(
        signals.iter().any(|s| s.ganger == target),
        "a shove off a ledge fires FallOccurred for the target (the shared 523 fall path)"
    );
    // The fall dropped Hp (the shared weight-free fall damage).
    assert!(
        hp_of(&app, target) < hp_before,
        "the fall drops the shoved target's Hp (the shared 523 fall damage)"
    );
}

// ── QA(3) — supported=>move (no fall, no wound) ────────────────────────────────

/// QA(3): a shove onto a SUPPORTED cell (an intact Present slab at the target's storey) moves
/// the target one cell and does NOT fall — no `FallOccurred`, no Hp loss.
#[test]
fn shove_onto_supported_cell_moves_without_falling() {
    let mut app = shove_app();
    let mut surface = SurfaceGrid::new();
    // Both the start (6,5,2) AND the destination (7,5,2) have a Present slab → supported → move.
    surface.set_slab(upper(6, 5, 2), SlabState::Present);
    surface.set_slab(upper(7, 5, 2), SlabState::Present);
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), upper(5, 5, 2), 0);
    let target = shove_ganger(app.world_mut(), upper(6, 5, 2), 1);
    app.update();
    let hp_before = hp_of(&app, target);

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(upper(7, 5, 2)),
        "a shove onto a supported (Present-slab) cell moves the target one cell, same storey"
    );
    assert!(
        fall_signals(&app).iter().all(|s| s.ganger != target),
        "a supported shove fires NO FallOccurred (no fall)"
    );
    assert_eq!(
        hp_of(&app, target),
        hp_before,
        "a supported shove deals NO damage (pure displacement — the shove itself never wounds)"
    );
}

// ── QA(4) — blocked=>no-op ─────────────────────────────────────────────────────

/// QA(4): a shove into a cell OCCUPIED by another ganger is a NO-OP — the target does not move.
#[test]
fn shove_into_an_occupied_cell_is_a_noop() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = shove_ganger(app.world_mut(), ground(5, 5), 0);
    let target = shove_ganger(app.world_mut(), ground(6, 5), 1);
    // A blocker standing on the destination cell (7,5) — the shove would push the target into it.
    let _blocker = shove_ganger(app.world_mut(), ground(7, 5), 1);
    // Settle so occupancy maintenance publishes the blocker's slot before the shove resolves.
    app.update();
    app.update();

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a shove into a ganger-occupied cell is a NO-OP (never shove into a solid)"
    );
    assert!(
        fall_signals(&app).iter().all(|s| s.ganger != target),
        "a blocked shove fires no fall"
    );
}

/// A direction sanity check for the pure verb — a diagonal shove pushes one cell on BOTH axes
/// away from the shover (no pinned magnitude, a geometry relation only).
#[test]
fn pure_verb_diagonal_pushes_one_cell_on_both_axes() {
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    // Shover at (5,5), target diagonally at (6,6) (SouthEast) → destination (7,7), supported.
    let outcome = resolve_shove(
        Position::new(ground(5, 5)),
        Position::new(ground(6, 6)),
        Entity::PLACEHOLDER,
        &surface,
        &occupancy,
    );
    assert_eq!(
        outcome,
        ShoveOutcome::Moved { dest: ground(7, 7) },
        "a diagonal shove pushes one cell on both axes directly away from the shover"
    );
    // Sanity: the direction helper agrees (attacker->target is SouthEast).
    assert_eq!(
        Direction::from_cells(Cell::new(5, 5), Cell::new(6, 6)),
        Some(Direction::SouthEast),
        "the shove direction is the attacker->target compass step"
    );
}
