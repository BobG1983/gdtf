//! The prefab preview-viewport **tile sprites** (GTW-515 C4.3 / C4.4) — the change-driven redraw
//! that populates the offscreen preview with a sprite per drawable cell (default-floor fill +
//! painted overrides) plus the translucent HOVER GHOST (red when the placement is illegal).
//!
//! Every sprite is a world-space [`Sprite`] (an atlas image over the editor's terrain sheet) on the
//! isolated [`preview_layer`] with the [`PreviewTile`] marker, positioned by
//! [`cell_center_world`]. So the dedicated preview camera (also on that layer) renders these and
//! ONLY these — never the editor's window/UI content (the `RenderLayers` isolation, C4.3).
//!
//! The redraw is CHANGE-DRIVEN (despawn-all + respawn) when any input the preview depends on
//! changes — the [`EditorMap`], the [`MapEditorSession`] (theme / size / selected tile), the
//! [`CurrentEditLevel`] storey, or the [`HoveredCell`] — so a static frame costs nothing. It reads
//! the SAME model + resolution the save path + the capture drive read (the theme palette, the
//! per-def atlas index resolved THE WAY THE PRESENTER DOES via [`terrain_atlas_index`], the shared
//! [`evaluate_placement`] legality), so the preview shows exactly what a save would write.

use bevy::{image::TextureAtlas, prelude::*};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    Cell,
    level::UuidThemeRegistry,
    metric::{CellLevel, Level},
    terrain::def::TerrainDefRegistry,
};

use crate::{
    canvas::CurrentEditLevel,
    editor_map::EditorMap,
    hovered_cell::HoveredCell,
    placement::{ProposedPlacement, evaluate_placement},
    preview::{
        coords::{CELL_WORLD, cell_center_world},
        target::{PreviewTile, preview_layer},
    },
    session::MapEditorSession,
    terrain_graphics::terrain_atlas_index,
    tile_atlas::TileAtlas,
};

/// The z-order of a base (floor / painted) tile sprite — below the hover ghost.
const TILE_Z: f32 = 0.0;

/// The z-order of the hover ghost — above the base tiles so it reads as an overlay.
const GHOST_Z: f32 = 1.0;

/// The hover ghost's tint when the placement is LEGAL — translucent white (a faint preview over
/// the cell). A framework layout const.
const GHOST_LEGAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.45);

/// The hover ghost's tint when the placement is ILLEGAL — translucent red (C4.4). A framework
/// layout const.
const GHOST_ILLEGAL: Color = Color::srgba(1.0, 0.2, 0.2, 0.55);

/// `Update` (in `Editing`): redraw the preview tiles when any input the preview depends on changed
/// (GTW-515 C4.3 / C4.4).
///
/// Despawns every existing [`PreviewTile`] and respawns a sprite per drawable cell of the current
/// edit level: an UNPAINTED cell draws the theme default-floor; a PAINTED cell draws its own tile.
/// Then, if a cell is hovered, draws the translucent HOVER GHOST over it — RED when
/// [`evaluate_placement`] rejects the (hovered cell, selected tile) placement, faint-white when
/// legal. Change-driven (guarded on `is_changed` of every input) so a static frame does no work.
///
/// All model borrows are `Option` (state-scoped — bevy-traps #1); no-ops until they + the
/// registries resolve.
#[expect(
    clippy::too_many_arguments,
    reason = "the preview redraw reads every model input it depends on (map, session, edit level, \
              hover) + the three shared registries (terrain defs, themes, tile roles) + the tile \
              atlas + Commands to (re)spawn; each is a distinct Bevy SystemParam and Bevy's \
              injection cannot reduce them without a wrapper resource that changes the crate API"
)]
pub(crate) fn redraw_preview_tiles(
    mut commands: Commands,
    map: Option<Res<EditorMap>>,
    session: Option<Res<MapEditorSession>>,
    edit_level: Option<Res<CurrentEditLevel>>,
    hovered: Option<Res<HoveredCell>>,
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    roles: Option<Res<TileRoles>>,
    atlas: Option<Res<TileAtlas>>,
    existing: Query<Entity, With<PreviewTile>>,
) {
    let (
        Some(map),
        Some(session),
        Some(edit_level),
        Some(hovered),
        Some(registry),
        Some(themes),
        Some(roles),
        Some(atlas),
    ) = (
        map, session, edit_level, hovered, registry, themes, roles, atlas,
    )
    else {
        return;
    };

    // CHANGE-DRIVEN: only redraw when an input the preview depends on changed (or the atlas just
    // loaded / registries just resolved — is_changed covers first-insert too).
    let dirty = map.is_changed()
        || session.is_changed()
        || edit_level.is_changed()
        || hovered.is_changed()
        || registry.is_changed()
        || themes.is_changed()
        || roles.is_changed()
        || atlas.is_changed();
    if !dirty {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let level = edit_level.level();
    let size = session.grid_size();
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    let theme = session.theme();
    let default_floor = session
        .default_floor()
        .or_else(|| themes.default_floor(&theme));

    // BASE layer: one sprite per drawable cell of the current edit level — painted tile if any,
    // else the theme default-floor. An unresolved tile (no atlas index) draws nothing (the clear
    // colour shows through), matching the presenter's fall-back-rather-than-panic behaviour.
    for y in 0..height {
        for x in 0..width {
            let cell = Cell::new(x, y);
            let slot = CellLevel::new(cell, level);
            let tile = map.tile_at_level(slot).or(default_floor);
            let Some(tile) = tile else {
                continue;
            };
            let Some(index) = terrain_atlas_index(&registry, &roles, &tile) else {
                continue;
            };
            spawn_tile_sprite(&mut commands, &atlas, cell, *index, Color::WHITE, TILE_Z);
        }
    }

    // HOVER GHOST: a translucent overlay on the hovered cell, RED when the placement is illegal.
    draw_hover_ghost(
        &mut commands,
        &atlas,
        &map,
        &registry,
        &roles,
        &session,
        &hovered,
        level,
    );
}

/// Draw the translucent hover ghost over the hovered cell (GTW-515 C4.4), tinted RED when
/// [`evaluate_placement`] rejects placing the session's selected tile there, faint-white when
/// legal. No-ops when nothing is hovered or no paint tile is selected (nothing to preview).
#[expect(
    clippy::too_many_arguments,
    reason = "the ghost needs the same model + registry inputs the base redraw resolved (map, \
              defs, roles, session, hover) plus the atlas + Commands to spawn one sprite; each is \
              a borrowed SystemParam slice threaded from the single redraw system"
)]
fn draw_hover_ghost(
    commands: &mut Commands,
    atlas: &TileAtlas,
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    roles: &TileRoles,
    session: &MapEditorSession,
    hovered: &HoveredCell,
    level: Level,
) {
    let Some(cell) = hovered.cell() else {
        return;
    };
    let Some(tile) = session.selected_tile() else {
        return;
    };
    let slot = CellLevel::new(cell, level);
    let placement = ProposedPlacement::new(slot, tile);
    let verdict = evaluate_placement(
        map,
        registry,
        session.theme(),
        &placement,
        session.grid_size(),
    );
    let tint = if verdict.is_illegal() {
        GHOST_ILLEGAL
    } else {
        GHOST_LEGAL
    };
    // The ghost shows the tile being placed (its resolved index), tinted; fall back to a plain
    // translucent quad (index 0) if the tile has no atlas index so the ghost still reads.
    let index = terrain_atlas_index(registry, roles, &tile).map_or(0, |i| *i);
    spawn_tile_sprite(commands, atlas, cell, index, tint, GHOST_Z);
}

/// Spawn one preview tile sprite at `cell` showing atlas `index`, tinted `tint`, at z-order `z` —
/// on the isolated [`preview_layer`] with the [`PreviewTile`] marker (GTW-515 C4.3). Uses a
/// world-space [`Sprite`] atlas image over the editor's terrain sheet (NOT a `bevy_ui` node — this
/// is world content the offscreen camera renders).
fn spawn_tile_sprite(
    commands: &mut Commands,
    atlas: &TileAtlas,
    cell: Cell,
    index: usize,
    tint: Color,
    z: f32,
) {
    let mut sprite = Sprite::from_atlas_image(
        atlas.image(),
        TextureAtlas {
            layout: atlas.layout(),
            index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_WORLD));
    sprite.color = tint;
    let world = cell_center_world(cell);
    commands.spawn((
        sprite,
        Transform::from_translation(world.extend(z)),
        preview_layer(),
        PreviewTile,
    ));
}
