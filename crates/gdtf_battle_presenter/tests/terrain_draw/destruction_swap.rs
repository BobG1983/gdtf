//! Destruction tile swaps: cover to rubble, slab to destroyed in place, and the
//! drawn-band predicate (AC3, GTW-367, GTW-519 C6).

use bevy::ecs::message::Messages;
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    occupancy_sync::{CoverDestroyed, SlabDestroyed},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

/// AC3 — a `CoverDestroyed { at }` on the active level swaps that cover cell's sprite to
/// the `rubble` tile (the `rubble` def's seeded rect, read from the `SpriteDefRegistry` —
/// GTW-665), and the other terrain sprites are untouched.
#[test]
fn cover_destroyed_swaps_the_cover_cell_to_rubble() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let wall_cell = Cell::new(8, 7);
    let cover_cell = Cell::new(9, 8);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);
    let cover_key = CellLevel::new(cover_cell, l0);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(wall_key, TerrainKind::Wall),
            TerrainPlacement::new(cover_key, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_key, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let wall_rect_before = sprite_rect_at(&mut app, wall_key);

    // Smash the cover cell.
    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(cover_key));
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "rubble"),
        "the destroyed cover cell's sprite must now carry the `rubble` def's rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        wall_rect_before,
        "the other (wall) terrain sprite must be untouched by the cover destruction",
    );
}

/// GTW-367 C6/C7 (POSITIVE) — a `SlabDestroyed { at }` on the active level, fed through the
/// REAL presenter plugin's `swap_destroyed_slab` reaction, swaps that slab cell's sprite to
/// the `slab_destroyed` tile (the def's seeded rect, read from the `SpriteDefRegistry` —
/// GTW-665) IN PLACE — the SAME `Entity` persists
/// (no despawn/respawn), the destroyed rect is DISTINCT from the intact slab rect, and the
/// neighbouring slab cell is untouched.
///
/// Drives the production `TopDownRendererPlugin` (no hand-mutated sprite in the arrange):
/// authors two `Present` slabs, fires `BattleReady` + `update()`s so the real draw spawns the
/// tiles, then writes a real `SlabDestroyed` to its `Messages` buffer and `update()`s again so
/// `swap_destroyed_slab` runs (the settle-before-read rule) before the assert reads the
/// resulting material rect + entity id.
#[test]
fn slab_destroyed_swaps_the_slab_cell_to_destroyed_in_place() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(4, 5);
    let other_slab_cell = Cell::new(6, 7);
    let l0 = Level::new(0);
    let slab_key = CellLevel::new(slab_cell, l0);
    let other_slab_key = CellLevel::new(other_slab_cell, l0);

    // Author two Present slabs (no walls/cover) + the in-progress witness. The destroyed
    // slab and an untouched neighbour, so the test proves the swap is targeted.
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key, SlabState::Present);
    surface.set_slab(other_slab_key, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // Fire the one-shot draw so the real plugin spawns the terrain tiles.
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

    // Precondition: the destroyed treatment is a REAL visible swap (distinct def rect).
    assert_ne!(
        def_rect(&defs, "slab_destroyed"),
        def_rect(&defs, "slab"),
        "the slab_destroyed def rect must differ from the intact slab rect (a real swap)",
    );

    // The slab cell starts on the intact slab tile, and we capture its Entity id for the C7
    // same-entity check.
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab cell's sprite must start on the intact slab def's rect",
    );
    let entity_before = sprite_entity_at(&mut app, slab_key);
    assert!(
        entity_before.is_some(),
        "the slab cell's terrain sprite must exist before the destruction",
    );
    let other_rect_before = sprite_rect_at(&mut app, other_slab_key);

    // Smash the slab — write a REAL message to the buffer the plugin registered, then settle.
    app.world_mut()
        .resource_mut::<Messages<SlabDestroyed>>()
        .write(SlabDestroyed::new(slab_key));
    app.update();

    // POSITIVE: the slab cell now renders the destroyed-slab def's rect (the intended content
    // actually renders — not merely "something changed").
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab_destroyed"),
        "the destroyed slab cell's sprite must now carry the `slab_destroyed` def's rect",
    );
    // C7: the SAME entity persists (in-place mutation, no despawn/respawn).
    assert_eq!(
        sprite_entity_at(&mut app, slab_key),
        entity_before,
        "the destroyed slab cell must be the SAME Entity after the swap (no despawn/respawn)",
    );
    // The neighbouring slab cell is untouched.
    assert_eq!(
        sprite_rect_at(&mut app, other_slab_key),
        other_rect_before,
        "the other (intact) slab sprite must be untouched by the slab destruction",
    );
}

/// GTW-519 C6 — a `CoverDestroyed` on a DRAWN LOWER storey swaps that cover cell to rubble
/// (the drawn-band predicate), while a `CoverDestroyed` STRICTLY ABOVE the active view level
/// is ignored (that terrain is not drawn).
///
/// At `ActiveLevel` 1: authors cover at `(9,8)` on storey 0 (a lower drawn storey) and cover
/// at `(9,8)` on storey 2 (above active — not drawn). Smashes both. Asserts the storey-0 cover
/// swapped to rubble, and the storey-2 cell has no drawn sprite to swap (None) — the above-active
/// destruction is a no-op.
#[test]
fn cover_destroyed_swaps_on_lower_storey_and_ignores_above_active() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let l2 = Level::new(2);
    let lower_cover = CellLevel::new(Cell::new(9, 8), l0);
    let above_cover = CellLevel::new(Cell::new(9, 8), l2);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower_cover, TerrainKind::Cover),
            TerrainPlacement::new(above_cover, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(lower_cover, low_cover_entry());
    cover_ledger.insert(above_cover, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
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

    // Precondition: the lower cover is drawn (a real tile to swap); the above-active cover is
    // NOT drawn (culled), so there is no tile there to begin with.
    assert_eq!(
        sprite_rect_at(&mut app, lower_cover),
        def_rect(&defs, "cover"),
        "the lower-storey cover must be drawn before the smash",
    );
    assert_eq!(
        sprite_rect_at(&mut app, above_cover),
        None,
        "the above-active cover must NOT be drawn (culled)",
    );

    // Smash BOTH cover cells.
    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(lower_cover));
    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(above_cover));
    app.update();

    // C6: the DRAWN lower-storey cover swapped to rubble.
    assert_eq!(
        sprite_rect_at(&mut app, lower_cover),
        def_rect(&defs, "rubble"),
        "a cover smashed on a DRAWN lower storey must swap to the rubble tile (C6)",
    );
    // The above-active destruction is ignored — still no drawn tile there.
    assert_eq!(
        sprite_rect_at(&mut app, above_cover),
        None,
        "a cover smashed STRICTLY ABOVE the active view level must be ignored (not drawn)",
    );
}
