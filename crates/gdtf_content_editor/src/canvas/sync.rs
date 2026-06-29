//! The [`sync_canvas`] `Update` system — initial build and theme/size-change rebuild of the
//! canvas grid (swept onto the UUID-keyed terrain model in GTW-495).
//!
//! Separated from the type definitions ([`super::types`]) and the interactivity systems
//! ([`super::paint`], [`super::ghost`]) so the rebuild logic has a single focused home.

use bevy::prelude::*;
use gdtf_battle_presenter::{TileIndex, TileRoles};
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_ui::theme::GdtfTheme;

use super::types::{CanvasBuiltFor, CanvasExtent, CanvasRoot, spawn_canvas};
use crate::{
    editor_map::EditorMap, session::MapEditorSession, terrain_graphics::terrain_atlas_index,
    tile_atlas::TileAtlas,
};

/// `Update` (in `Editing`): keep the central canvas in sync with the session — the INITIAL
/// build (C1/C2/C3) AND the theme/size-change rebuild (C4), in ONE system.
///
/// A single `Update` system (not an `OnEnter` build + an `Update` rebuild) because the canvas
/// reads the [`TileAtlas`], the [`TerrainDefRegistry`], the [`UuidThemeRegistry`], the
/// [`TileRoles`], and the [`MapEditorSession`] — several inserted via deferred `Commands`, so an
/// `OnEnter` build chained after those inserts would NOT see them (the GTW-421 command-flush
/// race). Running in `Update` sidesteps it.
///
/// It (re)builds when the `(grid_size, theme)` the canvas was last built for differs from the
/// session's current values — covering the first build, a size change (new extent + cell count —
/// C4), and a theme change (new default-floor fill — C4). PAINT PERSISTENCE across a rebuild
/// (GTW-426 C2): the authoritative paints live in the [`EditorMap`] model, NOT on the cell
/// entities, so a rebuild that respawns the cells does NOT lose them.
#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system's params are framework plumbing, not a wide function signature; the \
              canvas sync legitimately reads theme + atlas + the terrain registry + the theme \
              registry + the role table + the paint model + the existing roots + the rebuild \
              tracker"
)]
pub(crate) fn sync_canvas(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    atlas: Option<Res<TileAtlas>>,
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    roles: Option<Res<TileRoles>>,
    session: Option<Res<MapEditorSession>>,
    map: Option<Res<EditorMap>>,
    roots: Query<Entity, With<CanvasRoot>>,
    mut built_for: Local<Option<CanvasBuiltFor>>,
) {
    let (Some(atlas), Some(registry), Some(themes), Some(roles), Some(session), Some(map)) =
        (atlas, registry, themes, roles, session, map)
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
    // Resolve the default-floor tile sprite index for the active theme (C3): the theme's
    // default-floor TerrainUuid → its def's graphic → the TileRoles index. A theme with no
    // registered default floor leaves the fill unresolved — the canvas then draws empty cells
    // (boundary + dashes only) rather than panicking.
    let fill_index = resolve_default_floor_index(&registry, &themes, &roles, &session);

    for root in &roots {
        commands.entity(root).despawn();
    }
    let extent = CanvasExtent::from_grid(current.size);
    spawn_canvas(
        &mut commands,
        &theme,
        &atlas,
        &registry,
        &roles,
        &session,
        &map,
        extent,
        fill_index,
    );
    *built_for = Some(current);
}

/// Resolve the active theme's default-floor terrain's atlas index (C3), or [`None`] if the theme
/// has no resolvable default floor.
///
/// Prefers the session's resolved [`default_floor`](MapEditorSession::default_floor)
/// [`TerrainUuid`] (set on a theme selection / the seed); falls back to the
/// [`UuidThemeRegistry`]'s declared default floor for the theme. Both resolve through the
/// [`TerrainDefRegistry`] + [`TileRoles`] the way the presenter does.
fn resolve_default_floor_index(
    registry: &TerrainDefRegistry,
    themes: &UuidThemeRegistry,
    roles: &TileRoles,
    session: &MapEditorSession,
) -> Option<TileIndex> {
    let key: TerrainUuid = session
        .default_floor()
        .or_else(|| themes.default_floor(&session.theme()))?;
    terrain_atlas_index(registry, roles, &key)
}
