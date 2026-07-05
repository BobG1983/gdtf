//! `ViewMode` full-view toggle redraw + round-trip (GTW-521 C1/C2/C3).

use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::{
    BattleInProgress, BattleReady, Cell, CellLevel, CoverLedger, Level, SlabState, SurfaceGrid,
};

use super::harness::*;

/// GTW-521 C1/C2/C3 — with two storeys authored and `ActiveLevel = 0`:
/// [`ViewMode::DownToActive`] (the default) draws ONLY storey-0's slab (storey-1 culled),
/// flipping to [`ViewMode::FullView`] redraws the band and NOW draws storey-1's slab too, and
/// flipping BACK removes it again (the round-trip, C3).
///
/// Drives the REAL `draw_static_battlefield` system: the `ViewMode` change is its GTW-521
/// redraw trigger (added alongside the `ActiveLevel::is_changed` trigger), so the drawn set
/// changes with the toggle and no other input. The active level is held at 0 throughout, so
/// this isolates the `ViewMode` ceiling (C5 — the toggle does not move the active storey). The
/// slab on the upper storey is the peek-through-exempt REAL terrain fact, so it is the clean
/// storey-1 sprite to count.
#[test]
fn full_view_toggle_draws_upper_storeys_and_round_trips() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);

    // Empty occupancy: storey 0 draws its full floor field; storey 1 draws ONLY its authored
    // slab (peek-through, C2), which is the clean upper-storey sprite this test counts.
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(CellLevel::new(slab_cell, l0), SlabState::Present);
    surface.set_slab(CellLevel::new(slab_cell, l1), SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // ActiveLevel stays at 0 the WHOLE test; only the ViewMode changes. The renderer plugin
    // init_resource-s ViewMode(DownToActive) on build, so the default path runs first.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // DEFAULT (DownToActive) at active 0: storey 1 (strictly ABOVE active) is CULLED — ZERO
    // storey-1 sprites — exactly the GTW-519/520 behaviour (C1).
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

    // Flip to FullView: the ViewMode change re-runs the draw for the WHOLE stack, so storey
    // 1's slab NOW draws (C2) — despite the active level being unchanged at 0.
    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::FullView;
    app.update();
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        1,
        "in FullView, storey 1 must NOW draw its real terrain (the slab), regardless of the \
         active level (C2)",
    );
    // The lower storey's slab is still drawn (FullView never drops the band floor).
    assert_eq!(
        sprite_index_at(&mut app, CellLevel::new(slab_cell, l0)),
        tile_roles(&app).map(|r| *r.slab),
        "the storey-0 slab must STILL draw in FullView (the band floor is unchanged)",
    );

    // Flip BACK to DownToActive: the upper storey is culled again (the round-trip, C3).
    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::DownToActive;
    app.update();
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        0,
        "toggling back to DownToActive at active 0 must REMOVE storey 1 again (C3 round-trip)",
    );
}

/// Reads the presenter [`ViewMode`], or [`ViewMode::DownToActive`] if somehow absent (the
/// renderer plugin always inserts it — this keeps the read panic-free).
fn view_mode(app: &App) -> ViewMode {
    app.world()
        .get_resource::<ViewMode>()
        .copied()
        .unwrap_or(ViewMode::DownToActive)
}
