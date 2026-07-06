//! The in-place tile-material swap reactions: cover / slab destruction plus the
//! emplacement occupancy indicator.

use bevy::prelude::*;
use gdtf_battle_sim::{CoverDestroyed, EmplacementState, SlabDestroyed, TerrainCell};

use super::{
    active_level::{ActiveLevel, ViewMode},
    band::{cell_level_in_band, drawn_band},
    roles::TileRoles,
    static_draw::TerrainSprite,
};
use crate::TerrainFogMaterial;

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): swap
/// a destroyed cover cell's sprite to the RUBBLE tile.
///
/// Drains [`MessageReader<CoverDestroyed>`](gdtf_battle_sim::CoverDestroyed); for each
/// `CoverDestroyed { at }` WITHIN THE DRAWN BAND `[0..=active]` (GTW-519 C6 — the shared
/// [`drawn_band`] predicate, so a cover smashed on any drawn lower storey swaps too) it finds
/// the [`TerrainSprite`] at `at` and swaps its texture-atlas index to the `rubble`
/// [`TileIndex`](super::roles::TileIndex) (read from [`TileRoles`], never a literal). A
/// destruction on a storey
/// strictly ABOVE the active view level is ignored (that terrain is not drawn). Choice: SWAP
/// (not despawn) so the cell still reads as terrain (rubble) rather than a hole — AC3 asserts
/// the swap.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<TileRoles>`],
/// [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap re-indexes the tile's
/// material rather than its sprite), [`MessageReader<CoverDestroyed>`], and the
/// [`TerrainSprite`] / [`MeshMaterial2d`] query (to find the material at `at`) — no
/// [`Commands`] needed, the swap edits the material in place (no despawn / respawn).
pub fn swap_destroyed_cover(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<CoverDestroyed>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let band = drawn_band(*active, *view);
    let rubble = *roles.rubble;
    for event in destroyed.read() {
        // CoverDestroyed.at is a CellLevel; only act on cells WITHIN the drawn band
        // [0..=active] (GTW-519 C6 — a cover smashed on a DRAWN lower storey still swaps to
        // rubble; one strictly ABOVE the active view level is not drawn, so there is no tile
        // to re-index). SHARES the `drawn_band` helper with the draw loop so the two cannot
        // drift.
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        for (terrain, mat_handle) in &tiles {
            if terrain.at != event.at {
                continue;
            }
            // Re-index to the rubble tile — keep the entity (it still reads as terrain).
            // get_mut marks the material asset dirty so the UV-transform uniform re-uploads
            // next frame (mirroring the fog writer's in-place saturation edit). A tile whose
            // material was dropped (the terrain sheet was absent at draw time, so none
            // spawned) is simply not in the query; nothing to re-index.
            if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                material.atlas_index = rubble;
            }
        }
    }
}

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): swap a
/// destroyed-SLAB cell's sprite to the destroyed-slab tile.
///
/// The slab mirror of [`swap_destroyed_cover`] (GTW-367 C1/C3): it drains
/// [`MessageReader<SlabDestroyed>`](gdtf_battle_sim::SlabDestroyed); for each
/// `SlabDestroyed { at }` WITHIN THE DRAWN BAND `[0..=active]` (GTW-519 C6 — the shared
/// [`drawn_band`] predicate) it finds the [`TerrainSprite`] at `at` and swaps its
/// texture-atlas index to the `slab_destroyed` [`TileIndex`](super::roles::TileIndex) (read
/// from [`TileRoles`], never
/// a literal — the engineer's-choice destroyed-slab treatment, which the
/// [`TileRoles::slab_destroyed`] doc-comment describes and flags for art review). A
/// destruction on a storey strictly ABOVE the active view level is ignored (that terrain is
/// not drawn). Choice: SWAP (not despawn) so the cell still reads as terrain (rubble/debris)
/// rather than a hole, and so the SAME sprite `Entity` persists across the swap (the UI
/// mutate-not-respawn rule, C7) — exactly the cover path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<TileRoles>`],
/// [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap re-indexes the tile's material
/// rather than its sprite), [`MessageReader<SlabDestroyed>`], and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] query (to find the material at `at`) — no [`Commands`] needed, the
/// swap edits the material in place (no despawn / respawn).
///
/// [`TileRoles::slab_destroyed`]: super::roles::TileRoles::slab_destroyed
pub fn swap_destroyed_slab(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<SlabDestroyed>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let band = drawn_band(*active, *view);
    let slab_destroyed = *roles.slab_destroyed;
    for event in destroyed.read() {
        // SlabDestroyed.at is a CellLevel; only act on cells WITHIN the drawn band
        // [0..=active] (GTW-519 C6 — a slab smashed on a DRAWN lower storey swaps to its
        // destroyed tile; one strictly ABOVE the active view level is not drawn, so nothing
        // to re-index there). SHARES the `drawn_band` helper with the draw loop.
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        for (terrain, mat_handle) in &tiles {
            if terrain.at != event.at {
                continue;
            }
            // Re-index to the destroyed-slab tile — keep the entity (it still reads as
            // terrain). get_mut marks the material asset dirty so the UV-transform uniform
            // re-uploads next frame (mirroring swap_destroyed_cover's in-place edit). A tile
            // whose material was dropped (the terrain sheet was absent at draw time) is simply
            // not in the query; nothing to re-index.
            if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                material.atlas_index = slab_destroyed;
            }
        }
    }
}

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): swap a
/// weapon-emplacement's tile between its VACANT and OCCUPIED sprite as its
/// [`EmplacementState`](gdtf_battle_sim::EmplacementState) changes (GTW-543 — the
/// occupied-state visual indicator).
///
/// The state analogue of [`swap_destroyed_cover`] / [`swap_destroyed_slab`]: rather than a
/// one-shot destruction MESSAGE, it reacts to the sim's per-entity
/// [`Changed<EmplacementState>`](gdtf_battle_sim::EmplacementState) — the enter/exit toggle
/// (`apply_emplacement_toggle`) flips the emplacement entity's state Vacant↔Occupied, and this
/// system mirrors that onto the drawn tile. For each emplacement whose state CHANGED and whose
/// cell lies WITHIN THE DRAWN BAND `[0..=active]` (the shared `drawn_band` predicate, so an
/// emplacement manned on any drawn lower storey re-tints too; one strictly ABOVE the active view
/// level is not drawn, so there is no tile to re-index), it finds the [`TerrainSprite`] at that
/// cell and re-indexes its material to the `emplacement_occupied`
/// [`TileIndex`](super::roles::TileIndex) while
/// [`Occupied`](gdtf_battle_sim::EmplacementState::Occupied), or back to the `emplacement` tile
/// while [`Vacant`](gdtf_battle_sim::EmplacementState::Vacant) — both read from [`TileRoles`],
/// never a literal.
///
/// Choice: SWAP (not despawn / overlay) so the SAME sprite `Entity` persists across the state
/// change (the UI mutate-not-respawn rule, mirroring the two destruction swaps) and the cell
/// keeps reading as terrain. `Changed` fires on the FIRST observation too (the freshly-spawned
/// `Vacant` emplacement), which is inert — it re-indexes the already-`emplacement` tile to the
/// `emplacement` index (a no-op), so a battle-start pass never mis-tints an unmanned mount.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<ViewMode>`], [`Res<TileRoles>`],
/// [`ResMut<Assets<TerrainFogMaterial>>`] (the swap re-indexes the tile's material in place — the
/// destruction-swap precedent), the `Changed<EmplacementState>` sim query (the emplacement's
/// state + its [`TerrainCell`](gdtf_battle_sim::TerrainCell)), and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] query (to find the material at the emplacement's cell). No [`Commands`]
/// needed — the swap edits the material, never spawning / despawning.
pub fn indicate_emplacement_occupied(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    roles: Res<TileRoles>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    emplacements: Query<(&EmplacementState, &TerrainCell), Changed<EmplacementState>>,
    tiles: Query<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>,
) {
    let band = drawn_band(*active, *view);
    let vacant_index = *roles.emplacement;
    let occupied_index = *roles.emplacement_occupied;
    for (state, cell) in &emplacements {
        // TerrainCell derefs to the emplacement's CellLevel. Only act on cells WITHIN the drawn
        // band [0..=active] (the drawn_band predicate the destruction swaps share) — an
        // emplacement on a storey strictly above the active view level is not drawn.
        let at = **cell;
        if !cell_level_in_band(at, &band) {
            continue;
        }
        // The tile the manned/unmanned state resolves to: the OCCUPIED variant while manned,
        // the VACANT emplacement tile otherwise (a Vacant `Changed` first-observation re-indexes
        // to the same index it already carries — an inert no-op, never a mis-tint).
        let target = if state.is_occupied() {
            occupied_index
        } else {
            vacant_index
        };
        for (terrain, mat_handle) in &tiles {
            if terrain.at != at {
                continue;
            }
            // Re-index the tile's material in place — keep the entity (the mutate-not-respawn
            // rule). get_mut marks the material dirty so the UV-transform uniform re-uploads next
            // frame (mirroring the destruction swaps). A tile whose material was dropped (the
            // terrain sheet was absent at draw time) is simply not in the query; nothing to
            // re-index.
            if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                material.atlas_index = target;
            }
        }
    }
}
