//! Multi-level drawn band `[0..=active]`: redraw, cull, peek-through, and per-storey
//! Z (GTW-519 C1/C2/C3/C7).

use bevy::{app::App, ecs::message::Messages, transform::components::Transform};
use gdtf_battle_presenter::{ActiveLevel, TerrainSprite, cell_to_world};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

/// GTW-519 C1/C7 — raising `ActiveLevel` redraws the WHOLE drawn band `[0..=active]`
/// (multi-level, bottom-up), culling everything strictly above `active`; nothing above the
/// band is drawn.
///
/// Authors a `Present` slab on level 0 AND one on level 1 (mirroring skirmish's `(2,2,0)` +
/// `(2,2,1)`). At `ActiveLevel` 0 only the level-0 slab draws and the level-1 slab is CULLED
/// (strictly above active); after raising `ActiveLevel` to 1, BOTH slabs draw (level 0 is a
/// DRAWN lower storey now, not despawned — the pre-GTW-519 single-active-level behaviour is
/// replaced by the UFO:EU band). Every drawn sprite lies WITHIN the band (`z <= active`).
#[test]
fn raising_active_level_redraws_the_whole_drawn_band() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let slab0 = CellLevel::new(slab_cell, l0);
    let slab1 = CellLevel::new(slab_cell, l1);

    // Empty occupancy: the ground plane (storey 0) draws its full floor field; the upper
    // storey draws ONLY its authored slab (peek-through, C2). The band-scoping is proven by
    // the slab sprites' PRESENCE per level and by every drawn sprite lying within the band.
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab0, SlabState::Present);
    surface.set_slab(slab1, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident");
    let Some(roles) = roles else { return };

    // At active level 0: the level-0 slab draws; the level-1 slab is CULLED (above active).
    assert_eq!(
        sprite_index_at(&mut app, slab0),
        Some(*roles.slab),
        "the level-0 slab sprite must be present at active level 0",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab1),
        None,
        "the level-1 slab sprite (strictly ABOVE active) must be CULLED at active level 0",
    );
    // Every drawn sprite is within the band [0..=0]: nothing above active.
    assert!(
        all_sprites_within_band(&mut app, l0),
        "every terrain sprite must be within the drawn band [0..=0] before the change",
    );

    // Raise the active level to 1.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.update();

    // BOTH slabs now draw: level 1 is the active storey, level 0 is a DRAWN lower storey
    // (NOT despawned — the multi-level band redraw, C1/C7).
    assert_eq!(
        sprite_index_at(&mut app, slab1),
        Some(*roles.slab),
        "after raising to level 1, the level-1 (active) slab sprite must be present",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab0),
        Some(*roles.slab),
        "after raising to level 1, the level-0 slab sprite must STILL be present (a drawn \
         lower storey, not despawned)",
    );
    // Every drawn sprite is within the band [0..=1]: nothing above active.
    assert!(
        all_sprites_within_band(&mut app, l1),
        "after raising to level 1, every terrain sprite must be within the drawn band [0..=1]",
    );
}

/// Whether every `TerrainSprite` in the world lies WITHIN the drawn band `[0..=active]`
/// (`at.z <= active`) — the GTW-519 multi-level cull check (nothing strictly above active).
fn all_sprites_within_band(app: &mut App, active: Level) -> bool {
    let ceiling = i32::from(*active);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).all(|t| t.at.z <= ceiling)
}

/// The `Transform.translation.z` of the one `TerrainSprite` at `key`, if present — the C3
/// per-storey Z probe.
fn sprite_z_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, transform)| transform.translation.z)
}

/// GTW-519 C1 — the multi-level draw spawns terrain for EVERY storey in `[0..=active]`
/// (bottom-up) and NONE strictly above `active`.
///
/// Authors a `Present` slab on storeys 0, 1, and 2, sets `ActiveLevel` to 1, fires the draw,
/// and asserts: storey 0 (a lower drawn storey) draws its full floor field (`> 3` sprites),
/// storey 1 (the active storey) draws its full floor field, and storey 2 (strictly above
/// active) draws ZERO sprites (the hard cull). The multi-level presence proves the loop
/// iterates the whole band, not a single active storey.
#[test]
fn multi_level_draws_the_band_below_and_at_active_and_none_above() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let l2 = Level::new(2);

    // Empty occupancy: the ground floor draws a full floor field; upper storeys draw only the
    // real terrain we author (a slab) — the peek-through invariant (C2, proven separately).
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(CellLevel::new(cell, l0), SlabState::Present);
    surface.set_slab(CellLevel::new(cell, l1), SlabState::Present);
    surface.set_slab(CellLevel::new(cell, l2), SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // Active view level = 1: draw storeys 0 and 1, cull 2.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // The ground floor (storey 0) draws its FULL floor field (the ground plane) — proof the
    // band extends below active, not a single storey.
    let ground_count = terrain_sprite_count_on_level(&mut app, l0);
    assert!(
        ground_count > 3,
        "storey 0 (the ground floor, below active) must draw its full floor field in the \
         multi-level band; got {ground_count}",
    );
    // The active storey (an UPPER storey) draws ONLY its real terrain (the one authored slab),
    // NOT a full floor field — the peek-through invariant (C2) applies to every storey above
    // the ground plane, active or not. So it draws exactly 1 (the slab), far fewer than the
    // ground floor's field.
    let active_count = terrain_sprite_count_on_level(&mut app, l1);
    assert_eq!(
        active_count, 1,
        "storey 1 (active, an upper storey) must draw ONLY its real terrain (the slab), not a \
         floor field — peek-through applies to every non-ground storey; got {active_count}",
    );
    // Everything strictly ABOVE active is culled — ZERO sprites on storey 2.
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l2),
        0,
        "storey 2 (strictly above active) must draw NOTHING (the hard cull)",
    );
}

/// GTW-519 C2 (peek-through) — an open/empty UPPER-storey cell emits NO sprite, while the
/// same `(x, y)` on the storey BENEATH it DOES emit; a real terrain fact on the upper storey
/// still emits.
///
/// At `ActiveLevel` 1: authors a slab at `(5,5,0)` (ground floor) with NOTHING at `(5,5,1)`
/// (an empty upper cell — a floor gap), plus a wall at `(7,7,1)` (real upper terrain). Asserts
/// `(5,5,1)` emits NONE (peek-through — the storey-0 cell reads through), `(5,5,0)` emits Some,
/// and `(7,7,1)` emits Some (real terrain still draws on the upper storey).
#[test]
fn upper_storey_gap_peeks_through_to_the_storey_beneath() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let gap_lower = CellLevel::new(Cell::new(5, 5), l0);
    let gap_upper = CellLevel::new(Cell::new(5, 5), l1);
    let wall_upper = CellLevel::new(Cell::new(7, 7), l1);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(wall_upper, TerrainKind::Wall)],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(gap_lower, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // The empty upper cell emits NOTHING — the floor-gap reveals the storey beneath (C2).
    assert_eq!(
        sprite_index_at(&mut app, gap_upper),
        None,
        "an open/empty upper-storey cell must emit NO sprite (peek-through)",
    );
    // The same (x,y) on the storey BENEATH it DOES emit (its slab) — the revealed cell.
    assert_eq!(
        sprite_index_at(&mut app, gap_lower),
        Some(*roles.slab),
        "the storey-0 cell beneath the upper gap must still emit (peek-through reveals it)",
    );
    // Real terrain on the upper storey still draws.
    assert_eq!(
        sprite_index_at(&mut app, wall_upper),
        Some(*roles.wall),
        "a REAL upper-storey terrain cell (a wall) must still emit its sprite",
    );
}

/// GTW-519 C3 (per-storey Z) — a tile on storey *k* sits at `z == cell_to_world(_, k).z`, and
/// the same `(x, y)` on storey *k+1* draws at a STRICTLY GREATER z (painter's occlusion).
///
/// At `ActiveLevel` 1: authors a wall at `(4,4)` on BOTH storey 0 and storey 1, then reads the
/// two tiles' `Transform.z`. Asserts each equals its storey's `cell_to_world` z and that the
/// upper tile's z is strictly greater — the occlusion falls out of the existing per-storey Z.
#[test]
fn per_storey_z_orders_upper_over_lower() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(4, 4);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let lower = CellLevel::new(cell, l0);
    let upper = CellLevel::new(cell, l1);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower, TerrainKind::Wall),
            TerrainPlacement::new(upper, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let lower_z = sprite_z_at(&mut app, lower);
    let upper_z = sprite_z_at(&mut app, upper);
    assert_eq!(
        lower_z,
        Some(cell_to_world(cell, l0).z),
        "the storey-0 tile must sit at cell_to_world(cell, L0).z",
    );
    assert_eq!(
        upper_z,
        Some(cell_to_world(cell, l1).z),
        "the storey-1 tile must sit at cell_to_world(cell, L1).z",
    );
    let (Some(lower_z), Some(upper_z)) = (lower_z, upper_z) else {
        return;
    };
    assert!(
        upper_z > lower_z,
        "the upper storey's tile z ({upper_z}) must be STRICTLY greater than the lower's \
         ({lower_z}) — painter's-algorithm occlusion from the per-storey Z",
    );
}
