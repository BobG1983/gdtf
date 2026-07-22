//! The in-place tile-material swap reactions: cover / slab destruction plus the
//! emplacement occupancy indicator.

use bevy::prelude::*;
use gdtf_battle_sim::{
    emplacement::EmplacementState,
    entity::TerrainCell,
    occupancy_sync::{CoverDestroyed, SlabDestroyed},
    prelude::CellLevel,
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    band::{cell_level_in_band, drawn_band},
    restamp::{StampedGraphic, stamp_tile_quiet},
    roles::TileRole,
    static_draw::TerrainSprite,
    static_map::SpriteResolveCtx,
    treatment::{IsolateView, StoreyViewMode},
};
use crate::TerrainFogMaterial;

/// Retarget the ONE [`TerrainSprite`] at `at` to the sprite `role`'s resolved def —
/// the shared in-place swap body (GTW-665): the material's `image` / `atlas_layout` /
/// `atlas_index` re-resolve from the def's source (a missing def retargets to the
/// LOUD magenta marker — C4, warned by [`SpriteResolveCtx::resolved`]) and the entity's
/// [`Transform`] re-derives from the def's ANCHOR (C2 — the seeded center anchors make
/// this the unchanged cell center). The fog-driven `saturation` / `brightness` are
/// deliberately left alone (the Compose-stage fog writer owns them per frame). KEEPS the
/// entity (the UI mutate-not-respawn rule). The writes ride the ONE tick-quiet stamp
/// helper ([`stamp_tile_quiet`], GTW-666 — shared with the registry-change restamp):
/// a REAL retarget `get_mut`s the material so the UV-transform uniform re-uploads next
/// frame, an already-correct tile (e.g. the emplacement `Changed` first-observation)
/// writes nothing. The [`StampedGraphic`] records the role key so a later def
/// hot-reload restamps what the tile NOW shows. A tile whose material was dropped is
/// simply not in the query; nothing to re-index.
fn retarget_tile(
    at: CellLevel,
    role: TileRole,
    resolve: &SpriteResolveCtx,
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    tiles: &mut Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    for (terrain, mat_handle, mut transform, mut stamped) in tiles.iter_mut() {
        if terrain.at != at {
            continue;
        }
        stamp_tile_quiet(
            resolve,
            materials,
            role.as_key(),
            at,
            mat_handle,
            &mut transform,
        );
        stamped.set_if_neq(StampedGraphic::from_key(role.as_key()));
    }
}

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): swap
/// a destroyed cover cell's sprite to the RUBBLE tile.
///
/// Drains [`MessageReader<CoverDestroyed>`](gdtf_battle_sim::occupancy_sync::CoverDestroyed); for each
/// `CoverDestroyed { at }` WITHIN THE DRAWN BAND `[0..=active]` (GTW-519 C6 — the shared
/// [`drawn_band`] predicate, so a cover smashed on any drawn lower storey swaps too) it finds
/// the [`TerrainSprite`] at `at` and retargets its material to the
/// [`TileRole::Rubble`] sprite def (resolved through the GTW-665
/// [`SpriteResolveCtx`], never a hardcoded index). A
/// destruction on a storey
/// strictly ABOVE the active view level is ignored (that terrain is not drawn). Choice: SWAP
/// (not despawn) so the cell still reads as terrain (rubble) rather than a hole — AC3 asserts
/// the swap.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], the [`SpriteResolveCtx`]
/// bundle, [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap retargets the
/// tile's material rather than its sprite), [`MessageReader<CoverDestroyed>`], and the
/// [`TerrainSprite`] / [`MeshMaterial2d`] / [`Transform`] / [`StampedGraphic`] query (to
/// find the material at `at`, re-anchor it, and re-stamp the shown key — GTW-666) — no
/// [`Commands`] needed, the swap edits in place (no despawn / respawn).
pub fn swap_destroyed_cover(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<CoverDestroyed>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = drawn_band(*active, StoreyViewMode::new(*view, *isolate));
    for event in destroyed.read() {
        // CoverDestroyed.at is a CellLevel; only act on cells WITHIN the drawn band
        // [0..=active] (GTW-519 C6 — a cover smashed on a DRAWN lower storey still swaps to
        // rubble; one strictly ABOVE the active view level is not drawn, so there is no tile
        // to retarget). SHARES the `drawn_band` helper with the draw loop so the two cannot
        // drift.
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        retarget_tile(
            event.at,
            TileRole::Rubble,
            &resolve,
            &mut materials,
            &mut tiles,
        );
    }
}

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): swap a
/// destroyed-SLAB cell's sprite to the destroyed-slab tile.
///
/// The slab mirror of [`swap_destroyed_cover`] (GTW-367 C1/C3): it drains
/// [`MessageReader<SlabDestroyed>`](gdtf_battle_sim::occupancy_sync::SlabDestroyed); for each
/// `SlabDestroyed { at }` WITHIN THE DRAWN BAND `[0..=active]` (GTW-519 C6 — the shared
/// [`drawn_band`] predicate) it finds the [`TerrainSprite`] at `at` and retargets its
/// material to the [`TileRole::SlabDestroyed`] sprite def (resolved through the GTW-665
/// [`SpriteResolveCtx`], never a hardcoded index — the engineer's-choice destroyed-slab
/// treatment). A
/// destruction on a storey strictly ABOVE the active view level is ignored (that terrain is
/// not drawn). Choice: SWAP (not despawn) so the cell still reads as terrain (rubble/debris)
/// rather than a hole, and so the SAME sprite `Entity` persists across the swap (the UI
/// mutate-not-respawn rule, C7) — exactly the cover path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], the [`SpriteResolveCtx`]
/// bundle, [`ResMut<Assets<TerrainFogMaterial>>`] (GTW-348 — the swap retargets the
/// tile's material rather than its sprite), [`MessageReader<SlabDestroyed>`], and the
/// [`TerrainSprite`] / [`MeshMaterial2d`] / [`Transform`] / [`StampedGraphic`] query —
/// no [`Commands`] needed, the swap edits in place (no despawn / respawn).
pub fn swap_destroyed_slab(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<SlabDestroyed>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = drawn_band(*active, StoreyViewMode::new(*view, *isolate));
    for event in destroyed.read() {
        // SlabDestroyed.at is a CellLevel; only act on cells WITHIN the drawn band
        // [0..=active] (GTW-519 C6 — a slab smashed on a DRAWN lower storey swaps to its
        // destroyed tile; one strictly ABOVE the active view level is not drawn, so nothing
        // to retarget there). SHARES the `drawn_band` helper with the draw loop.
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        retarget_tile(
            event.at,
            TileRole::SlabDestroyed,
            &resolve,
            &mut materials,
            &mut tiles,
        );
    }
}

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): swap a
/// weapon-emplacement's tile between its VACANT and OCCUPIED sprite as its
/// [`EmplacementState`](gdtf_battle_sim::emplacement::EmplacementState) changes (GTW-543 — the
/// occupied-state visual indicator).
///
/// The state analogue of [`swap_destroyed_cover`] / [`swap_destroyed_slab`]: rather than a
/// one-shot destruction MESSAGE, it reacts to the sim's per-entity
/// [`Changed<EmplacementState>`](gdtf_battle_sim::emplacement::EmplacementState) — the enter/exit toggle
/// (`apply_emplacement_toggle`) flips the emplacement entity's state Vacant↔Occupied, and this
/// system mirrors that onto the drawn tile. For each emplacement whose state CHANGED and whose
/// cell lies WITHIN THE DRAWN BAND `[0..=active]` (the shared `drawn_band` predicate, so an
/// emplacement manned on any drawn lower storey re-tints too; one strictly ABOVE the active view
/// level is not drawn, so there is no tile to retarget), it finds the [`TerrainSprite`] at that
/// cell and retargets its material to the [`TileRole::EmplacementOccupied`] sprite def
/// while [`Occupied`](gdtf_battle_sim::emplacement::EmplacementState::Occupied), or back to the
/// [`TileRole::Emplacement`] def while
/// [`Vacant`](gdtf_battle_sim::emplacement::EmplacementState::Vacant) — both resolved through the
/// GTW-665 [`SpriteResolveCtx`], never a hardcoded index.
///
/// Choice: SWAP (not despawn / overlay) so the SAME sprite `Entity` persists across the state
/// change (the UI mutate-not-respawn rule, mirroring the two destruction swaps) and the cell
/// keeps reading as terrain. `Changed` fires on the FIRST observation too (the freshly-spawned
/// `Vacant` emplacement), which is inert — it retargets the already-`emplacement` tile to the
/// `emplacement` def (a no-op), so a battle-start pass never mis-tints an unmanned mount.
///
/// Param-only (`bevy-traps.md` #7): [`Res<ActiveLevel>`], [`Res<ViewMode>`], the
/// [`SpriteResolveCtx`] bundle,
/// [`ResMut<Assets<TerrainFogMaterial>>`] (the swap retargets the tile's material in place — the
/// destruction-swap precedent), the `Changed<EmplacementState>` sim query (the emplacement's
/// state + its [`TerrainCell`](gdtf_battle_sim::entity::TerrainCell)), and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] / [`Transform`] / [`StampedGraphic`] query (to find the material at the
/// emplacement's cell). No [`Commands`] needed — the swap edits in place, never spawning /
/// despawning.
pub fn indicate_emplacement_occupied(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    emplacements: Query<(&EmplacementState, &TerrainCell), Changed<EmplacementState>>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = drawn_band(*active, StoreyViewMode::new(*view, *isolate));
    for (state, cell) in &emplacements {
        // TerrainCell derefs to the emplacement's CellLevel. Only act on cells WITHIN the drawn
        // band [0..=active] (the drawn_band predicate the destruction swaps share) — an
        // emplacement on a storey strictly above the active view level is not drawn.
        let at = **cell;
        if !cell_level_in_band(at, &band) {
            continue;
        }
        // The tile the manned/unmanned state resolves to: the OCCUPIED variant while manned,
        // the VACANT emplacement tile otherwise (a Vacant `Changed` first-observation retargets
        // to the def it already draws — an inert no-op, never a mis-tint).
        let role = if *state.is_occupied() {
            TileRole::EmplacementOccupied
        } else {
            TileRole::Emplacement
        };
        retarget_tile(at, role, &resolve, &mut materials, &mut tiles);
    }
}
