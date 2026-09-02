use bevy::{app::App, math::Vec3};
use gdtf_battle_sim::{
    cover::CoverLedger,
    entity::TerrainPieceKind,
    march::{MarchDir, MarchGrids, MarchKind, march_vector},
    prelude::{CellLevel, Level, OccupancyGrid, SimPos},
    surface::{SlabState, SurfaceGrid},
    tuning::{CombatTuning, PerStoreyDamage},
    vertical::VerticalLinkGraph,
};

use super::harness::*;

#[test]
fn destroyed_slab_stays_non_pathable_and_los_flies_through() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let _faller = spawn_faller(app.world_mut(), 1);
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    assert!(
        probe_stops_on_slab(&app),
        "an intact slab must STOP a probe ray at the z-boundary (the baseline)"
    );
    let links_before = app.world().resource::<VerticalLinkGraph>().links().count();

    destroy_slab_and_settle(&mut app, 1, TerrainPieceKind::Slab);

    assert_eq!(
        app.world()
            .resource::<SurfaceGrid>()
            .slab_state(&CellLevel::new(column_cell(), Level::new(1))),
        SlabState::Absent,
        "this app holds no def registry, so the smashed slab leaves nothing behind and the \
         cell reads Absent",
    );
    assert!(
        !probe_stops_on_slab(&app),
        "a destroyed slab must let the probe ray fly THROUGH (LOS passthrough — unchanged)"
    );
    let links_after = app.world().resource::<VerticalLinkGraph>().links().count();
    assert_eq!(
        links_before, links_after,
        "a destroyed slab adds NO vertical link — the hole stays non-pathable (unchanged)"
    );
}

fn probe_stops_on_slab(app: &App) -> bool {
    let occupancy = app.world().resource::<OccupancyGrid>();
    let surface = app.world().resource::<SurfaceGrid>();
    let cover = app.world().resource::<CoverLedger>();
    let tuning = app.world().resource::<CombatTuning>();
    let muzzle = SimPos::new(COL_X as f32 + 0.5, COL_Y as f32 + 0.5, 0.5);
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy,
            surface,
            cover,
        },
        tuning,
        CellLevel::new(column_cell(), Level::new(0)),
        |_| false,
    );
    matches!(result.kind, MarchKind::Slab)
}
