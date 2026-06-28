//! The [`sync_canvas`] `Update` system — initial build and theme/size-change rebuild of the
//! canvas grid.
//!
//! Separated from the type definitions ([`super::types`]) and the interactivity systems
//! ([`super::paint`], [`super::ghost`]) so the rebuild logic has a single focused home.

use bevy::prelude::*;
use gdtf_battle_sim::level::{ThemeCatalogRegistry, TileAtlasIndex};
use gdtf_ui::theme::GdtfTheme;

use super::types::{CanvasBuiltFor, CanvasExtent, CanvasRoot, spawn_canvas};
use crate::{editor_map::EditorMap, session::MapEditorSession, tile_atlas::TileAtlas};

/// `Update` (in `Editing`): keep the central canvas in sync with the session — the INITIAL
/// build (C1/C2/C3) AND the theme/size-change rebuild (C4), in ONE system.
///
/// A single `Update` system (not an `OnEnter` build + an `Update` rebuild) because the canvas
/// reads the [`TileAtlas`], the [`ThemeCatalogRegistry`], and the [`MapEditorSession`] — all
/// inserted via deferred `Commands` (the session + atlas on `OnEnter(Editing)`), so an
/// `OnEnter` build chained after their inserts would NOT see them (the GTW-421 / GTW-422
/// command-flush race). Running in `Update` sidesteps it: the resources are present the first
/// frame in `Editing`.
///
/// It (re)builds when the `(grid_size, theme)` the canvas was last built for differs from the
/// session's current values — covering the first build (tracker `None`), a size change (new
/// extent + cell count — C4), and a theme change (new default-floor fill — C4). It despawns the
/// existing [`CanvasRoot`] subtree, then rebuilds the boundary + cell grid + fill, parenting the
/// new root under the [`CanvasRegion`](crate::CanvasRegion)'s [`ScrollListArea`] (the GTW-421
/// parenting rule). The last-built `(size, theme)` is tracked in a [`Local`] so an unrelated
/// session mutation (a palette selection) never triggers a needless rebuild.
///
/// PAINT PERSISTENCE across a rebuild (GTW-426 C2): the authoritative paints live in the
/// [`EditorMap`] model, NOT on the cell entities — so a size/theme rebuild that despawns + respawns
/// the cells does NOT lose them. As each cell is respawned its fill is taken from the model first
/// (a painted cell shows its painted tile) and falls back to the theme default-floor only where the
/// model has no entry. So a paint survives an unrelated rebuild rather than being silently wiped.
#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system's params are framework plumbing, not a wide function signature; the \
              canvas sync legitimately reads theme + atlas + the two catalog sources + the paint \
              model + the existing roots + the rebuild tracker"
)]
pub(crate) fn sync_canvas(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    atlas: Option<Res<TileAtlas>>,
    registry: Option<Res<ThemeCatalogRegistry>>,
    session: Option<Res<MapEditorSession>>,
    map: Option<Res<EditorMap>>,
    roots: Query<Entity, With<CanvasRoot>>,
    mut built_for: Local<Option<CanvasBuiltFor>>,
) {
    let (Some(atlas), Some(registry), Some(session), Some(map)) = (atlas, registry, session, map)
    else {
        return;
    };
    let current = CanvasBuiltFor {
        size:  session.grid_size(),
        theme: session.theme(),
    };
    if *built_for == Some(current) {
        // Already built for this size + theme — nothing relevant changed.
        return;
    }
    // Resolve the default-floor tile sprite index for the active theme (C3). Prefer the
    // session's resolved default-floor key (written when a theme is selected); fall back to the
    // catalog's own declared default-floor key on first open (the dropdown has not fired yet,
    // so the session key is still `None`). A theme with no catalog leaves the fill unresolved —
    // the canvas then draws empty cells (boundary + dashes only) rather than panicking.
    let fill_index = resolve_default_floor_index(&registry, &session);

    for root in &roots {
        commands.entity(root).despawn();
    }
    let extent = CanvasExtent::from_grid(current.size);
    spawn_canvas(
        &mut commands,
        &theme,
        &atlas,
        &registry,
        &session,
        &map,
        extent,
        fill_index,
    );
    *built_for = Some(current);
}

/// Resolve the active theme's default-floor tile's atlas index (C3), or [`None`] if the theme
/// has no catalog / no resolvable default floor.
///
/// Prefers the session's resolved [`default_floor`](MapEditorSession::default_floor)
/// [`TileKey`](gdtf_battle_sim::level::TileKey) (set on a theme selection); falls back to the
/// catalog's declared default-floor key (the
/// first-open case, before the dropdown fires). Both resolve through the active theme's catalog
/// to the [`CatalogTile`](gdtf_battle_sim::level::CatalogTile)'s
/// [`TileAtlasIndex`](gdtf_battle_sim::level::TileAtlasIndex).
fn resolve_default_floor_index(
    registry: &ThemeCatalogRegistry,
    session: &MapEditorSession,
) -> Option<TileAtlasIndex> {
    let catalog = registry.catalog(session.theme())?;
    let tile = match session.default_floor() {
        Some(key) => catalog.tile(key),
        None => catalog.default_floor(),
    }?;
    Some(tile.atlas_index)
}
