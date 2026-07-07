//! Per-def `TerrainGraphicKey` resolution beats the `TerrainKind` role default +
//! the optional footfall (GTW-493, GTW-469).

use bevy::ecs::message::Messages;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

/// GTW-493 C1 (PIN-DISCRIMINATING, the in-engine evidence) — two cells of the SAME
/// `TerrainKind` (`Cover`) but DIFFERENT `presenter_kind.graphic_name` resolve to DISTINCT
/// atlas indices, driven through the REAL `draw_static_battlefield` system.
///
/// Both cells are authored `TerrainKind::Cover` in the occupancy grid, so a
/// kind-keyed-only resolution would draw them
/// IDENTICALLY at the `cover` def. The sim-spawned per-def `TerrainGraphicKey` ("cover" vs
/// "rubble") is what makes them DIFFER: cell A resolves to the `cover` def's rect, cell B
/// to the `rubble` def's rect (GTW-665 — read from the SEEDED defs, never a literal).
/// Reverting the presenter to kind-keyed-only resolution makes both
/// the `cover` rect — identical — and this test FAILS.
///
/// Occlusion-aware (a visible+laid-out node can still draw nothing): the assertion reads
/// the resolved material rect actually carried by the spawned sprite at each cell
/// (`sprite_rect_at`), and settles a frame (the one-shot draw) before reading.
#[test]
fn per_def_graphic_distinguishes_same_kind_cells() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let cover_a = CellLevel::new(Cell::new(8, 7), l0);
    let cover_b = CellLevel::new(Cell::new(9, 8), l0);

    // Both cells are the SAME TerrainKind::Cover in the occupancy grid — so the role-table
    // default keyed on TerrainKind would draw both at roles.cover.
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(cover_a, TerrainKind::Cover),
            TerrainPlacement::new(cover_b, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_a, low_cover_entry());
    cover_ledger.insert(cover_b, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: SAME kind, DISTINCT graphic_names.
    spawn_terrain_entity(&mut app, cover_a, "cover", None);
    spawn_terrain_entity(&mut app, cover_b, "rubble", None);

    // Fire the one-shot draw and settle.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    // Precondition: the two defs cut DISTINCT sheet rects (so a "different" assertion is
    // meaningful rather than vacuously true). Read from the SEEDED defs, never a literal.
    let cover_rect = def_rect(&defs, "cover");
    let rubble_rect = def_rect(&defs, "rubble");
    assert!(
        cover_rect.is_some() && rubble_rect.is_some(),
        "the `cover` + `rubble` seeded defs must resolve with Sheet rects",
    );
    assert_ne!(
        cover_rect, rubble_rect,
        "the `cover` and `rubble` def rects must differ (else the pin is vacuous)",
    );

    let rect_a = sprite_rect_at(&mut app, cover_a);
    let rect_b = sprite_rect_at(&mut app, cover_b);

    // POSITIVE: each cell resolves to ITS OWN def graphic, not the shared TerrainKind default.
    assert_eq!(
        rect_a, cover_rect,
        "cell A (graphic_name \"cover\") must draw the `cover` def's sheet rect",
    );
    assert_eq!(
        rect_b, rubble_rect,
        "cell B (graphic_name \"rubble\") must draw the `rubble` def's sheet rect, NOT the \
         shared TerrainKind::Cover default",
    );
    // The DISCRIMINATING clause: two same-TerrainKind cells draw DIFFERENT sprites — exactly
    // what kind-keyed-only resolution could not do.
    assert_ne!(
        rect_a, rect_b,
        "two cells of the SAME TerrainKind but DIFFERENT graphic_name must draw DIFFERENT \
         sprites (kind-keyed-only resolution would make them identical and fail here)",
    );
}

/// GTW-469 C3 (PIN-DISCRIMINATING, the in-engine evidence) — an NS-wall cell and an EW-wall
/// cell, BOTH `TerrainKind::Wall` but DIFFERENT `presenter_kind.graphic_name` (`"wall"` vs
/// `"wall_ew"`), resolve to DISTINCT atlas indices, driven through the REAL
/// `draw_static_battlefield` system.
///
/// Both cells are authored `TerrainKind::Wall` in the occupancy grid, so a
/// kind-keyed-only resolution would draw them
/// IDENTICALLY at the `wall` def — orientation is presentation-only, so the sim semantics ARE
/// identical (C5). The sim-spawned per-def `TerrainGraphicKey` (`"wall"` vs `"wall_ew"`) is what
/// makes the SPRITES differ: the NS cell resolves to the `wall` def's rect, the EW cell to the
/// `wall_ew` def's rect (the row-2 rotated tile — GTW-665, read from the SEEDED defs). Reverting
/// the presenter to kind-keyed-only resolution makes both
/// the `wall` rect — identical — and this test FAILS.
///
/// Occlusion-aware (a visible+laid-out node can still draw nothing): the assertion reads the
/// resolved material rect actually carried by the spawned sprite at each cell
/// (`sprite_rect_at`), and settles a frame (the one-shot draw) before reading.
#[test]
fn ns_and_ew_wall_resolve_to_distinct_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let wall_ns = CellLevel::new(Cell::new(8, 7), l0);
    let wall_ew = CellLevel::new(Cell::new(9, 8), l0);

    // Both cells are the SAME TerrainKind::Wall in the occupancy grid — so the role-table
    // default keyed on TerrainKind would draw both at roles.wall (identical LOS/move blocking).
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(wall_ns, TerrainKind::Wall),
            TerrainPlacement::new(wall_ew, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: SAME TerrainKind::Wall, DISTINCT graphic_names —
    // the NS def's "wall" vs the GTW-469 EW def's "wall_ew".
    spawn_terrain_entity(&mut app, wall_ns, "wall", None);
    spawn_terrain_entity(&mut app, wall_ew, "wall_ew", None);

    // Fire the one-shot draw and settle.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    // Precondition: the two wall-orientation defs cut DISTINCT sheet rects (so a
    // "different" assertion is meaningful rather than vacuously true).
    let wall_rect = def_rect(&defs, "wall");
    let wall_ew_rect = def_rect(&defs, "wall_ew");
    assert!(
        wall_rect.is_some() && wall_ew_rect.is_some(),
        "the `wall` + `wall_ew` seeded defs must resolve with Sheet rects",
    );
    assert_ne!(
        wall_rect, wall_ew_rect,
        "the `wall` (NS) and `wall_ew` (EW) def rects must differ (else the pin is vacuous)",
    );

    let rect_ns = sprite_rect_at(&mut app, wall_ns);
    let rect_ew = sprite_rect_at(&mut app, wall_ew);

    // POSITIVE: each cell resolves to ITS OWN orientation graphic, not the shared Wall default.
    assert_eq!(
        rect_ns, wall_rect,
        "the NS-wall cell (graphic_name \"wall\") must draw the `wall` def's sheet rect",
    );
    assert_eq!(
        rect_ew, wall_ew_rect,
        "the EW-wall cell (graphic_name \"wall_ew\") must draw the `wall_ew` def's sheet rect, \
         NOT the shared TerrainKind::Wall default",
    );
    // The DISCRIMINATING clause: two same-TerrainKind::Wall cells draw DIFFERENT (perpendicular)
    // sprites — exactly what role-table-only resolution (keyed on TerrainKind) could not do.
    assert_ne!(
        rect_ns, rect_ew,
        "an NS-wall cell and an EW-wall cell (SAME TerrainKind::Wall, DIFFERENT graphic_name) \
         must draw DIFFERENT sprites (kind-keyed-only resolution would make them identical)",
    );
}

/// GTW-493 C2 — a `Slab` cell's footfall is read from the def's `presenter_kind` (an
/// OPTIONAL `FootfallSound`), and an ABSENT footfall is handled with NO panic and a
/// documented default.
///
/// Drives the REAL `draw_static_battlefield` over two slab cells: one whose terrain entity
/// names a footfall, one whose entity OMITS it (the `None` footfall — the documented
/// silent default). The draw reading the footfall must NOT panic on either, and both slab
/// cells must still render their per-def graphic (here both `"slab"`), proving the footfall
/// read is a non-fatal presentation hook layered onto the same draw.
#[test]
fn slab_footfall_optional_is_read_without_panic() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let slab_with = CellLevel::new(Cell::new(4, 5), l0);
    let slab_without = CellLevel::new(Cell::new(6, 7), l0);

    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_with, SlabState::Present);
    surface.set_slab(slab_without, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // One slab def names a footfall; the other OMITS it (the documented silent default).
    spawn_terrain_entity(&mut app, slab_with, "slab", Some("step_metal"));
    spawn_terrain_entity(&mut app, slab_without, "slab", None);

    // Fire the one-shot draw and settle — the draw reading the OPTIONAL footfall must not
    // panic for either the present-footfall or the absent-footfall slab.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };
    let slab_rect = def_rect(&defs, "slab");
    assert!(
        slab_rect.is_some(),
        "the `slab` seeded def must resolve with a Sheet rect"
    );

    // Both slab cells render their per-def `slab` graphic — the absent-footfall slab draws
    // exactly like the present-footfall one (the footfall is a non-visual hook; its absence
    // is the silent default, never a missing tile).
    assert_eq!(
        sprite_rect_at(&mut app, slab_with),
        slab_rect,
        "the slab cell WITH a footfall must render the `slab` graphic",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_without),
        slab_rect,
        "the slab cell WITHOUT a footfall must STILL render the `slab` graphic (absent \
         footfall is the documented silent default, not a missing tile)",
    );
}
