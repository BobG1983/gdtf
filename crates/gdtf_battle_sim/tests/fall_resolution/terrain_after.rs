//! The destroyed slab's terrain contract: non-pathable, LOS flies through (QA 8).

use bevy::{app::App, math::Vec3};
use gdtf_battle_sim::{
    cover::CoverLedger,
    march::{MarchKind, march_vector},
    prelude::{CellLevel, Level, OccupancyGrid, SimPos},
    surface::{SlabState, SurfaceGrid},
    tuning::{CombatTuning, PerStoreyDamage},
    vertical::VerticalLinkGraph,
};

use super::harness::*;

// ── QA(8): regression — a hole is non-pathable + LOS flies through ────────────

/// QA(8): destroying the slab under a faller does NOT make the hole walkable — the
/// `VerticalLinkGraph` is untouched (no vertical link added) — AND LOS flies THROUGH the
/// hole (the shared `march_vector`: BEFORE a probe STOPS on the intact slab; AFTER it climbs
/// THROUGH to a non-Slab result). The pathing + LOS behavior for a destroyed slab is exactly
/// the GTW-365 contract — GTW-523 adds the fall WITHOUT changing either.
#[test]
fn destroyed_slab_stays_non_pathable_and_los_flies_through() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let _faller = spawn_faller(app.world_mut(), 1);
    // An intact slab at the boundary above the ground cell (level 1 = floor of storey 1).
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    // BEFORE: a probe ray straight UP through the boundary STOPS on the intact slab.
    assert!(
        probe_stops_on_slab(&app),
        "an intact slab must STOP a probe ray at the z-boundary (the baseline)"
    );
    let links_before = app.world().resource::<VerticalLinkGraph>().links().count();

    // Destroy the slab (the same signal apply_falls consumed) — a faller at level 1 is on the
    // ground floor (level 0 supports below it), so this exercises the destruction path.
    destroy_slab_and_settle(&mut app, 1);

    // AFTER: the destroyed slab is transparent to the march — the probe climbs THROUGH the
    // hole to a non-Slab result (LOS flies through — UNTOUCHED by GTW-523).
    assert!(
        !probe_stops_on_slab(&app),
        "a destroyed slab must let the probe ray fly THROUGH (LOS passthrough — unchanged)"
    );
    // The VerticalLinkGraph is UNTOUCHED — a destroyed slab adds no vertical link, so the
    // hole stays non-pathable (movement is unchanged — GTW-523 adds no pathing).
    let links_after = app.world().resource::<VerticalLinkGraph>().links().count();
    assert_eq!(
        links_before, links_after,
        "a destroyed slab adds NO vertical link — the hole stays non-pathable (unchanged)"
    );
}

/// March a probe ray straight UP from just under the boundary through the column cell,
/// reading the LIVE grids — returns whether it STOPS on an intact slab (the shared
/// `march_vector` `has_los` uses; `MarchKind::Slab` fires ONLY on an intact slab).
fn probe_stops_on_slab(app: &App) -> bool {
    let occupancy = app.world().resource::<OccupancyGrid>();
    let surface = app.world().resource::<SurfaceGrid>();
    let cover = app.world().resource::<CoverLedger>();
    let tuning = app.world().resource::<CombatTuning>();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the column x/y are tiny grid coords, exact in f32"
    )]
    let muzzle = SimPos::new(COL_X as f32 + 0.5, COL_Y as f32 + 0.5, 0.5);
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let result = march_vector(
        muzzle,
        dir,
        occupancy,
        surface,
        cover,
        tuning,
        CellLevel::new(column_cell(), Level::new(0)),
        |_| false,
    );
    matches!(result.kind, MarchKind::Slab)
}
