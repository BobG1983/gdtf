//! Emplacement dedicated tile + occupied in-place swap (GTW-543).

use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_sim::{
    BattleInProgress, BattleReady, Cell, CellLevel, CoverLedger, EmplacementState, Level,
    SurfaceGrid, TerrainCell, TerrainGraphicKey, TerrainKind, TerrainPlacement,
};

use super::harness::*;

/// Spawns ONE sim-side weapon-EMPLACEMENT terrain entity at `key` (GTW-543) carrying its
/// per-def [`TerrainGraphicKey`] (`"emplacement"`) AND the [`EmplacementState`] the enter/exit
/// toggle flips — mirroring exactly what the sim's `setup_battle` spawns for an `Emplacement`
/// terrain def (a terrain entity with the graphic key + `EmplacementState::Vacant`). Returns the
/// spawned [`Entity`] so the test can flip its state to drive the `Changed<EmplacementState>`
/// swap.
fn spawn_emplacement_entity(app: &mut App, key: CellLevel) -> bevy::ecs::entity::Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(key),
            TerrainGraphicKey::new("emplacement".to_owned()),
            EmplacementState::Vacant,
        ))
        .id()
}

/// GTW-543 (PRESENTER draw, POSITIVE) — a weapon-emplacement terrain entity (a
/// `TerrainGraphicKey` of `"emplacement"`) draws its DEDICATED emplacement tile, distinct from
/// the generic `cover` tile, through the REAL `draw_static_battlefield` system.
///
/// Both cells are the SAME `TerrainKind::Emplacement` in the occupancy grid; the sim-spawned
/// per-def `TerrainGraphicKey` is what picks the tile. The emplacement cell resolves to
/// `roles.emplacement` (NOT `roles.cover`), so an emplacement reads distinctly from a chest-high
/// crate — the ticket's "a distinct glyph/color/tile". Occlusion-aware: it reads the resolved
/// material `atlas_index` actually carried by the spawned sprite (`sprite_index_at`) after
/// settling the one-shot draw.
#[test]
fn emplacement_draws_its_own_distinct_tile() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let emp_cell = CellLevel::new(Cell::new(8, 7), l0);
    let cover_cell = CellLevel::new(Cell::new(9, 8), l0);

    // Author the emplacement cell as TerrainKind::Emplacement + a plain cover cell alongside, so
    // the test proves the emplacement draws its OWN tile rather than the shared cover default.
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(emp_cell, TerrainKind::Emplacement),
            TerrainPlacement::new(cover_cell, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(emp_cell, low_cover_entry());
    cover_ledger.insert(cover_cell, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: the emplacement graphic + a plain cover graphic.
    spawn_emplacement_entity(&mut app, emp_cell);
    spawn_terrain_entity(&mut app, cover_cell, "cover", None);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the emplacement tile is DISTINCT from the cover tile (else the pin is vacuous).
    assert_ne!(
        *roles.emplacement, *roles.cover,
        "the `emplacement` and `cover` role indices must differ (else the distinct-tile pin is \
         vacuous)",
    );

    // POSITIVE: the emplacement cell resolves to its OWN dedicated tile, not the cover default.
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement),
        "the emplacement cell must draw the dedicated `emplacement` tile, NOT the generic cover \
         tile",
    );
    // The plain cover cell still draws the cover tile (the emplacement graphic did not leak).
    assert_eq!(
        sprite_index_at(&mut app, cover_cell),
        Some(*roles.cover),
        "the plain cover cell must still draw the `cover` tile",
    );
}

/// GTW-543 (PRESENTER occupied-indicator, POSITIVE) — a `Changed<EmplacementState>` from
/// Vacant→Occupied swaps the emplacement cell's tile IN PLACE to the `emplacement_occupied` tile
/// (the same `Entity`), and vacating swaps it back — through the REAL
/// `indicate_emplacement_occupied` system.
///
/// Spawns one emplacement terrain entity (drawn `Vacant` on the `emplacement` tile), flips its
/// `EmplacementState` to `Occupied` and settles (the settle-before-read rule), asserts the tile
/// re-indexes to `emplacement_occupied` on the SAME entity (mutate-not-respawn), then flips it
/// back to `Vacant` and asserts it restores to `emplacement`. Mirrors the destruction-swap
/// tests' shape but driven by a component state change rather than a destruction message.
#[test]
fn occupying_an_emplacement_swaps_its_tile_in_place() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let emp_cell = CellLevel::new(Cell::new(4, 5), l0);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(emp_cell, TerrainKind::Emplacement)],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(emp_cell, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    let emplacement = spawn_emplacement_entity(&mut app, emp_cell);

    // Fire the one-shot draw so the real plugin spawns the emplacement tile (Vacant).
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the occupied tile is a REAL visible swap (distinct atlas index).
    assert_ne!(
        *roles.emplacement_occupied, *roles.emplacement,
        "the `emplacement_occupied` tile index must differ from the vacant `emplacement` index \
         (a real swap)",
    );

    // The emplacement starts on the vacant tile; capture its Entity id for the same-entity check.
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement),
        "an unmanned emplacement's sprite must start on the vacant `emplacement` tile",
    );
    let entity_before = sprite_entity_at(&mut app, emp_cell);
    assert!(
        entity_before.is_some(),
        "the emplacement cell's terrain sprite must exist before it is manned",
    );

    // Man the emplacement — flip the sim state Vacant→Occupied (the enter act's effect), then
    // settle so `indicate_emplacement_occupied` runs on the Changed<EmplacementState>.
    let occupied = app.world_mut().get_mut::<EmplacementState>(emplacement);
    assert!(
        occupied.is_some(),
        "the spawned emplacement entity must carry EmplacementState",
    );
    let Some(mut state) = occupied else { return };
    *state = EmplacementState::Occupied;
    app.update();

    // POSITIVE: the manned emplacement now renders the OCCUPIED tile (the intended content
    // actually renders — not merely "something changed").
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement_occupied),
        "a manned emplacement's sprite must swap to the `emplacement_occupied` tile",
    );
    // The SAME entity persists (in-place mutation, no despawn/respawn).
    assert_eq!(
        sprite_entity_at(&mut app, emp_cell),
        entity_before,
        "the manned emplacement cell must be the SAME Entity after the swap (no despawn/respawn)",
    );

    // Vacate — flip Occupied→Vacant (the exit act's effect), then settle: the tile restores.
    let vacate = app.world_mut().get_mut::<EmplacementState>(emplacement);
    assert!(
        vacate.is_some(),
        "the emplacement entity must still carry EmplacementState",
    );
    let Some(mut state) = vacate else { return };
    *state = EmplacementState::Vacant;
    app.update();
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement),
        "a vacated emplacement's sprite must restore to the vacant `emplacement` tile",
    );
}
