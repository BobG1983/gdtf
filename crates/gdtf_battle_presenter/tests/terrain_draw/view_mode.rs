use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

#[test]
fn full_view_toggle_draws_upper_storeys_and_round_trips() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);

    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(CellLevel::new(slab_cell, l0), SlabState::Present);
    surface.set_slab(CellLevel::new(slab_cell, l1), SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the renderer plugin seeds the default ViewMode (DownToActive)",
    );
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        0,
        "in DownToActive at active 0, storey 1 (above active) must draw NOTHING (C1)",
    );

    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::FullView;
    app.update();
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        1,
        "in FullView, storey 1 must NOW draw its real terrain (the slab), regardless of the \
         active level (C2)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, CellLevel::new(slab_cell, l0)),
        sprite_defs(&app).and_then(|defs| def_rect(&defs, "slab")),
        "the storey-0 slab must STILL draw in FullView (the band floor is unchanged)",
    );

    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::DownToActive;
    app.update();
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        0,
        "toggling back to DownToActive at active 0 must REMOVE storey 1 again (C3 round-trip)",
    );
}

fn view_mode(app: &App) -> ViewMode {
    app.world()
        .get_resource::<ViewMode>()
        .copied()
        .unwrap_or(ViewMode::DownToActive)
}
