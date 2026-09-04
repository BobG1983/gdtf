use bevy::{app::App, ecs::system::RunSystemOnce};
use gdtf_battle_presenter::{PlaybackCursor, register_playback};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::Faction,
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, TerrainKind},
};

use super::{ShownCoverLedger, ShownOccupancyGrid, promote_shown_cover, promote_shown_occupancy};

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn shadow_app() -> App {
    let mut app = App::new();
    register_playback(&mut app);
    app.insert_resource(ActLog::default());
    app.init_resource::<ShownOccupancyGrid>();
    app.init_resource::<ShownCoverLedger>();
    app
}

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
        ActWitnesses::unseen(),
    ));
}

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

fn run_occupancy_promote(app: &mut App) {
    let ran = app.world_mut().run_system_once(promote_shown_occupancy);
    assert!(ran.is_ok(), "the occupancy promote must run in the fixture");
}

fn run_cover_promote(app: &mut App) {
    let ran = app.world_mut().run_system_once(promote_shown_cover);
    assert!(ran.is_ok(), "the cover promote must run in the fixture");
}

fn wall_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
        TerrainPieceKind::Wall,
    )
}

#[test]
fn occupancy_shadow_freezes_then_promotes() {
    let mut app = shadow_app();
    let wall = ground(4, 4);

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

#[test]
fn cover_shadow_freezes_then_promotes() {
    let mut app = shadow_app();
    let wall = ground(5, 5);

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
