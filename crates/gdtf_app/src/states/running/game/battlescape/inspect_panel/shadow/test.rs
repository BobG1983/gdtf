//! Freeze/promote proof for the inspect panel's cursor-time SHADOWS (GTW-762): the grid and
//! ledger snapshots stay FROZEN during closed-gate playback and PROMOTE the instant the
//! cursor catches up.
//!
//! Mirrors the bare-cursor playback harness: a focused app with the playback cursor and a
//! hand-built act log, driven one promote-system run at a time — NO wall-clock waits. The
//! gate is closed by growing the log (the head moves past the cursor) and opened by advancing
//! the cursor to the head, so each assertion is a statement about the code, not about timing.

use bevy::{app::App, ecs::system::RunSystemOnce};
use gdtf_battle_presenter::{PlaybackCursor, register_playback};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Faction,
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, TerrainKind},
};

use super::{ShownCoverLedger, ShownOccupancyGrid, promote_shown_cover, promote_shown_occupancy};

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The focused shadow app: the playback cursor + tuning + `Played<M>` buffers (via
/// `register_playback`) and a hand-built empty act log. No render stack — the promote systems
/// read only the cursor, the log, and the live resources.
fn shadow_app() -> App {
    let mut app = App::new();
    register_playback(&mut app);
    app.insert_resource(ActLog::default());
    app.init_resource::<ShownOccupancyGrid>();
    app.init_resource::<ShownCoverLedger>();
    app
}

/// Append one entry to the act log — grows the head, so a cursor still at the start is no
/// longer caught up (the gate closes).
fn close_gate(app: &mut App) {
    let entity = app.world_mut().spawn_empty().id();
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("the fixture inserts an act log");
    };
    log.append(RecordedAct::new(
        entity,
        ActProvenance::Clock,
        ActDeed::TurnBegan {
            now_active: Faction::new(0),
        },
    ));
}

/// Advance the cursor to the log head — the cursor catches up, so the gate opens.
fn catch_up(app: &mut App) {
    let head = {
        let Some(log) = app.world().get_resource::<ActLog>() else {
            unreachable!("the fixture inserts an act log");
        };
        log.head()
    };
    let Some(mut cursor) = app.world_mut().get_resource_mut::<PlaybackCursor>() else {
        unreachable!("register_playback inits the cursor");
    };
    while cursor.shown() < head {
        cursor.advance_past_shown();
    }
}

/// Run the occupancy promote once.
fn run_occupancy_promote(app: &mut App) {
    let ran = app.world_mut().run_system_once(promote_shown_occupancy);
    assert!(ran.is_ok(), "the occupancy promote must run in the fixture");
}

/// Run the cover promote once.
fn run_cover_promote(app: &mut App) {
    let ran = app.world_mut().run_system_once(promote_shown_cover);
    assert!(ran.is_ok(), "the cover promote must run in the fixture");
}

/// A low-HP wall cover entry for the ledger discriminator.
fn wall_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

/// The occupancy-grid shadow FREEZES while closed-gate playback runs and PROMOTES the instant
/// the cursor catches up — the GTW-762 clause-6 grid proof.
///
/// Discriminates on the terrain kind at one cell: state A is all-Open, state B marks the cell
/// a `Wall`. While the gate is shut the shadow keeps reading A even though the LIVE grid has
/// already reached B; the moment the cursor catches up it reads B.
#[test]
fn occupancy_shadow_freezes_then_promotes() {
    let mut app = shadow_app();
    let wall = ground(4, 4);

    // State A: an empty (all-Open) live grid. Gate open (empty log) -> promote copies A.
    app.insert_resource(OccupancyGrid::new());
    run_occupancy_promote(&mut app);
    assert_eq!(
        app.world()
            .resource::<ShownOccupancyGrid>()
            .grid()
            .terrain(&wall),
        TerrainKind::Open,
        "the shadow promotes the live grid while caught up (state A: Open)",
    );

    // The sim reaches state B (the cell becomes a Wall) AND the gate closes (the log grows
    // past the still-at-start cursor) — the closed-gate playback window.
    {
        let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
            unreachable!("the grid was inserted above");
        };
        grid.set_terrain(wall, TerrainKind::Wall);
    }
    close_gate(&mut app);
    run_occupancy_promote(&mut app);
    assert_eq!(
        app.world()
            .resource::<ShownOccupancyGrid>()
            .grid()
            .terrain(&wall),
        TerrainKind::Open,
        "the shadow stays FROZEN at state A while the gate is shut, even though live is B",
    );

    // The cursor catches up -> the gate opens -> the shadow promotes to B.
    catch_up(&mut app);
    run_occupancy_promote(&mut app);
    assert_eq!(
        app.world()
            .resource::<ShownOccupancyGrid>()
            .grid()
            .terrain(&wall),
        TerrainKind::Wall,
        "the shadow PROMOTES to state B the instant the cursor catches up",
    );
}

/// The cover-ledger shadow FREEZES while closed-gate playback runs and PROMOTES the instant
/// the cursor catches up — the GTW-762 clause-6 ledger proof.
///
/// Discriminates on whether a cell has a ledger entry: state A is the empty ledger, state B
/// inserts a wall entry at the cell.
#[test]
fn cover_shadow_freezes_then_promotes() {
    let mut app = shadow_app();
    let wall = ground(5, 5);

    // State A: empty ledger. Gate open -> promote copies A (no entry at the cell).
    app.insert_resource(CoverLedger::new());
    run_cover_promote(&mut app);
    assert!(
        app.world()
            .resource::<ShownCoverLedger>()
            .ledger()
            .peek(&wall)
            .is_none(),
        "the shadow promotes the live ledger while caught up (state A: no entry)",
    );

    // The sim reaches state B (an entry appears at the cell) AND the gate closes.
    {
        let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
            unreachable!("the ledger was inserted above");
        };
        ledger.insert(wall, wall_entry());
    }
    close_gate(&mut app);
    run_cover_promote(&mut app);
    assert!(
        app.world()
            .resource::<ShownCoverLedger>()
            .ledger()
            .peek(&wall)
            .is_none(),
        "the shadow stays FROZEN at state A while the gate is shut, even though live is B",
    );

    // The cursor catches up -> the gate opens -> the shadow promotes to B.
    catch_up(&mut app);
    run_cover_promote(&mut app);
    assert!(
        app.world()
            .resource::<ShownCoverLedger>()
            .ledger()
            .peek(&wall)
            .is_some(),
        "the shadow PROMOTES to state B the instant the cursor catches up",
    );
}
