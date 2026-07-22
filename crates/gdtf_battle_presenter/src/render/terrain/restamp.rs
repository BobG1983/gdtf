//! The GTW-666 sprite-def hot-reload **restamp**: when the
//! [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry)
//! changes (a `.spritedef.ron` re-save rebuilds it through the family redrive),
//! every ALREADY-DRAWN terrain tile re-resolves the graphic it is stamped with
//! and re-applies texture / rect / anchor IN PLACE — the same entity, never a
//! despawn + respawn (the GTW-631 registry-change restamp shape).
//!
//! Tick-quiet by construction (the GTW-627 tick-quiet write discipline, shared with the
//! swap reactions through [`stamp_tile_quiet`]): an UNCHANGED def writes nothing —
//! no material `get_mut`, no `Transform` re-dirty, zero `AssetEvent::Modified` —
//! so a no-op registry touch or an unrelated-def change leaves unaffected tiles'
//! change ticks untouched. The vertical-link tiles need no restamp: their pooled
//! draw ([`draw_vertical_links`](super::link_draw::draw_vertical_links)) already
//! re-resolves every frame.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::CellLevel;
use gdtf_content_families::sprites::SpriteName;

use super::{static_draw::TerrainSprite, static_map::SpriteResolveCtx};
use crate::{TerrainFogMaterial, cell_to_world};

/// The graphic NAME a drawn terrain tile is currently STAMPED with — the key its
/// pixels were last resolved from (the sim's per-def
/// [`TerrainGraphicKey`](gdtf_battle_sim::piece::TerrainGraphicKey), the
/// [`TileRole`](super::roles::TileRole) fallback key, or a swap reaction's role
/// key such as `rubble`).
///
/// Written at draw time and by every in-place swap, so the GTW-666 restamp can
/// re-resolve exactly what the tile SHOWS (a rubble-swapped tile restamps against
/// the new `rubble` def, not the cell's underlying occupancy fact). A named
/// newtype component over the family's [`SpriteName`] key (no-bare-types rule 1),
/// read through [`Deref`].
#[derive(Component, Debug, Clone, PartialEq, Eq, Deref)]
pub struct StampedGraphic(SpriteName);

impl StampedGraphic {
    /// Stamp a graphic key (the resolution key the tile's pixels came from).
    #[must_use]
    pub fn from_key(name: &str) -> Self {
        Self(SpriteName::new(name.to_owned()))
    }

    /// The stamped key as the `&str` the def resolution consumes.
    pub(super) fn as_key(&self) -> &str {
        self.0.as_str()
    }
}

/// Tick-quietly apply the resolved stamp for graphic `name` to ONE drawn tile at
/// `at` — THE shared write helper (GTW-666) behind both the swap reactions'
/// retarget and the registry-change restamp.
///
/// Resolves `name` through the ONE GTW-665 resolution
/// ([`SpriteResolveCtx::resolved`] — a missing def yields the LOUD magenta marker)
/// and applies it with the GTW-627 compare-before-write discipline: the material's
/// `image` / `atlas_layout` / `atlas_index` are compared through the immutable
/// [`Assets::get`] (no resource tick) and written via [`Assets::get_mut`] ONLY on
/// a real diff (a `get_mut` queues the `AssetEvent::Modified` that re-uploads the
/// tile's uniform — an unchanged def must not); the fog-owned `saturation` /
/// `brightness` are never touched (the Compose-stage fog writer owns them). The
/// `Transform` re-derives from the def's ANCHOR (GTW-665 C2) and is written only
/// when the translation actually moved. Takes the [`ResMut`] / [`Mut`] WRAPPERS
/// (the GTW-627 / GTW-568 lesson: a bare `&mut` deref at the call site would mark
/// the store / component changed every run; the wrapper's immutable deref reads
/// without dirtying, and only the real-diff assignment goes through `DerefMut`).
pub(super) fn stamp_tile_quiet(
    resolve: &SpriteResolveCtx,
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    name: &str,
    at: CellLevel,
    material: &MeshMaterial2d<TerrainFogMaterial>,
    transform: &mut Mut<'_, Transform>,
) {
    let (target, offset) = resolve.resolved(name, &at);
    let stale = materials.get(material.id()).is_some_and(|current| {
        current.image != target.image
            || current.atlas_layout != target.atlas_layout
            || current.atlas_index != target.atlas_index
    });
    if stale && let Some(mut current) = materials.get_mut(material.id()) {
        current.image = target.image;
        current.atlas_layout = target.atlas_layout;
        current.atlas_index = target.atlas_index;
    }
    let translation = cell_to_world(at.cell(), at.level()) + offset.extend(0.0);
    if transform.translation != translation {
        transform.translation = translation;
    }
}

/// `Update` ([`PresenterSystems`](super::active_level::PresenterSystems)`::Scene`,
/// gated `resource_exists::<BattleInProgress>` + the resolution bundle, ordered
/// after the draw + swap writers): restamp every drawn terrain tile when the
/// [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry)
/// changed this tick (GTW-666).
///
/// The trigger is the registry's change tick (`SpriteResolveCtx::defs_changed`)
/// — a `.spritedef.ron` re-save (hot-reload during a battle, or a SPRITE-mode
/// save in the editor host) rebuilds the registry through the family redrive, and
/// this system re-resolves each tile's [`StampedGraphic`] key against the new
/// defs, re-applying texture / rect / anchor through the ONE tick-quiet write
/// helper (`stamp_tile_quiet`): a tile whose def actually changed is re-stamped
/// in place (mutate-not-respawn — the same entity survives), a tile whose def is
/// unchanged is left with its change ticks untouched (the tick-quiet witness).
/// On any frame the registry did NOT change it returns without touching a tile.
///
/// The `Transform` in the query is written through the [`Mut`] wrapper's deref
/// only on a real move (a plain compare-then-assign — reading through [`Mut`]'s
/// immutable deref never dirties), so an all-quiet pass leaves every tile's
/// change ticks exactly as they were.
///
/// Param-only (`bevy-traps.md` #7): the [`SpriteResolveCtx`] resolution bundle,
/// [`ResMut<Assets<TerrainFogMaterial>>`] (the in-place material retarget — the
/// swap reactions' write helper), and the stamped-tile query.
pub fn restamp_tiles_on_def_change(
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut tiles: Query<(
        &TerrainSprite,
        &StampedGraphic,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
    )>,
) {
    // The single trigger: the registry's change tick (relative to THIS system's
    // last run — an insert_resource rebuild and a bare set_changed both fire it).
    if !resolve.defs_changed() {
        return;
    }
    for (tile, stamped, material, mut transform) in tiles.iter_mut() {
        stamp_tile_quiet(
            &resolve,
            &mut materials,
            stamped.as_key(),
            tile.at,
            material,
            &mut transform,
        );
    }
}
