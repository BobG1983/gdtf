use bevy::platform::collections::HashSet;
use gdtf_battle_presenter::{ActiveLevel, CrossLevelBadgeKind, CrossLevelSignals, TerrainSprite};
use gdtf_battle_sim::{
    falls::StoreysFallen,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    visibility::SquadVisibility,
};

use super::harness::{settle, signals_app};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

#[test]
fn destroyed_slab_bordering_drawn_floor_emits_drop_depth() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(2)));

    let hole = key(5, 5, 2);
    let floor = key(6, 5, 2);

    let mut surface = SurfaceGrid::new();
    surface.set_slab(floor, SlabState::Present);
    surface.set_slab(hole, SlabState::Absent);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(OccupancyGrid::default());

    let explored: HashSet<CellLevel> = [hole, floor].into_iter().collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(explored.clone(), explored));

    app.world_mut().spawn(TerrainSprite { at: floor });

    settle(&mut app);

    let badges: Vec<CrossLevelBadgeKind> = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(hole.cell())
        .to_vec();
    assert!(
        badges
            .iter()
            .any(|b| matches!(*b, CrossLevelBadgeKind::DropDepth { storeys } if storeys == StoreysFallen::new(2))),
        "a destroyed slab bordering drawn floor must emit DropDepth(2), got {badges:?}",
    );
}
