//! Who falls when a slab dies: the occupant falls, the roof occupant does not,
//! multi-storey pass-through, other-level immunity, and stair bracing (QA 1-4).

use gdtf_battle_sim::{CellLevel, Level, OccupancyGrid, PerStoreyDamage, SlabState, SurfaceGrid};

use super::harness::*;

// ── QA(1) + QA(3): faller predicate keys off level == N, NOT N+1 ──────────────

/// QA(1): a 3-storey column, faller on level 2, `SlabDestroyed` at (cell, 2) → the faller's
/// `Position` drops to the ground (highest support below) and the storeys distance is 2.
/// QA(3): an EXPLICIT roof-decoy faller at level 3 (== `destroyed_level` + 1, the ROOF — the
/// WRONG actor) does NOT move, proving the predicate keys off `level == N`, not `N + 1`.
#[test]
fn faller_on_destroyed_slab_level_falls_roof_occupant_does_not() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    // The faller STANDING on the floor of storey 2 (Position.level == 2).
    let faller = spawn_faller(app.world_mut(), 2);
    // The decoy on the ROOF (level 3 == destroyed_level + 1) — the wrong actor; must not fall.
    let roof_decoy = spawn_faller(app.world_mut(), 3);
    // An empty surface grid (no intermediate slabs) → the faller drops to the ground (0).
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 2);

    // QA(1): the level-2 faller dropped to the ground (0), a 2-storey fall.
    assert_eq!(
        level_of(&app, faller),
        0,
        "the level-2 faller drops to the ground"
    );
    // QA(3): the roof-decoy at level 3 (destroyed_level + 1) did NOT move — the predicate
    // keys off level == N, not N + 1.
    assert_eq!(
        level_of(&app, roof_decoy),
        3,
        "the roof occupant (level+1) is the WRONG actor and must NOT fall"
    );
    // QA(1): exactly ONE FallOccurred, for the level-2 faller, storeys == 2, from 2 → 0.
    let signals = fall_signals(&app);
    assert_eq!(
        signals.len(),
        1,
        "exactly one fall (the roof decoy did not fall)"
    );
    let signal = signals[0];
    assert_eq!(signal.ganger, faller);
    assert_eq!(signal.from_level, Level::new(2));
    assert_eq!(signal.to_level, Level::new(0));
    assert_eq!(*signal.storeys, 2, "start 2 → land 0 is a 2-storey fall");
}

// ── QA(2): multi-storey drop through an Absent intermediate ───────────────────

/// QA(2): a faller at level 4 over an `Absent` intermediate (level 2, open air) and an
/// intact `Present` floor at level 1 falls THROUGH the open air and lands on the level-1
/// slab (not the ground) — a 3-storey drop.
#[test]
fn multi_storey_drop_through_absent_lands_on_first_present() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(6));
    let faller = spawn_faller(app.world_mut(), 4);
    let mut surface = SurfaceGrid::new();
    // level 1: an intact Present floor (the landing). levels 2/3: Absent (open air).
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 4);

    assert_eq!(
        level_of(&app, faller),
        1,
        "the faller falls through the Absent intermediate and lands on the Present level-1 slab"
    );
    let signals = fall_signals(&app);
    assert_eq!(signals.len(), 1);
    assert_eq!(
        *signals[0].storeys, 3,
        "start 4 → land 1 is a 3-storey fall"
    );
}

// ── QA(3): a ganger on a different level does not move ─────────────────────────

/// QA(3): a faller on a DIFFERENT level than the destroyed slab does not move and no fall
/// fires. (The roof-decoy case above covers level+1 specifically; this covers a level BELOW
/// the destroyed slab.)
#[test]
fn ganger_on_different_level_does_not_fall() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    // A faller on level 1 — BELOW the level-3 destroyed slab, a different storey.
    let elsewhere = spawn_faller(app.world_mut(), 1);
    // Intact floor at level 1 so `elsewhere` is on solid ground (not itself falling).
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 3);

    assert_eq!(
        level_of(&app, elsewhere),
        1,
        "a ganger on a different level does not move"
    );
    assert!(
        fall_signals(&app).is_empty(),
        "no fall fires for a ganger off the destroyed level"
    );
}

// ── QA(4): a stair lower-endpoint occupant is braced ──────────────────────────

/// QA(4): a faller standing on an authored STAIR tile at the destroyed slab's level is
/// BRACED — the stair supports it, so it does NOT fall through its own stair (no drop, no
/// damage, no signal).
#[test]
fn stair_lower_endpoint_occupant_is_braced() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let braced = spawn_faller(app.world_mut(), 2);
    app.insert_resource(SurfaceGrid::new());
    // Mark the faller's cell an authored stair tile — the C3 brace.
    let mut occupancy = OccupancyGrid::new();
    occupancy.mark_stair_cell(CellLevel::new(column_cell(), Level::new(2)));
    app.insert_resource(occupancy);
    let hp_before = hp_of(&app, braced);

    destroy_slab_and_settle(&mut app, 2);

    assert_eq!(
        level_of(&app, braced),
        2,
        "a braced stair occupant does not fall"
    );
    assert_eq!(
        hp_of(&app, braced),
        hp_before,
        "a braced occupant takes no fall damage"
    );
    assert!(
        fall_signals(&app).is_empty(),
        "a braced occupant fires no FallOccurred"
    );
}
