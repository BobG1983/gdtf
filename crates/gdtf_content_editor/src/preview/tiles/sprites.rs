//! The preview tile-sprite TREATMENT TABLE (GTW-594 C2) — the editor's class → pixels
//! mapping (tints + overlay patterns + z layout) — plus the sprite-spawn helpers and the
//! translucent hover ghost.
//!
//! Three unmistakable per-cell classes at any zoom (GTW-594 C2): AUTHORED-HERE (active
//! storey, full-bright art), EXISTS-BELOW (the categorical ghost — cool blue-grey
//! hue+alpha tint PLUS the 2×2-block stipple overlay), and EMPTY (the faint void grid on
//! unpainted active cells). Only the [`StoreyTreatment::Active`] class renders full-bright
//! (the A1 law's editor half, unit-tested in the sibling `test` module).

use bevy::{image::TextureAtlas, prelude::*};
use gdtf_battle_presenter::StoreyTreatment;
use gdtf_battle_sim::{
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::TerrainDefRegistry,
};

use crate::{
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

/// The z-order of a base (floor / painted) tile sprite on the GROUND storey — below the
/// hover ghost. Higher storeys are lifted by [`STOREY_Z_GAP`] per storey so an upper floor
/// draws in FRONT of a lower one (the painter's-algorithm occlusion the presenter's
/// per-storey Z gives).
pub(super) const TILE_Z: f32 = 0.0;

/// The per-storey z-lift for a multi-storey draw (GTW-532) — each storey above the ground
/// is drawn `STOREY_Z_GAP` further forward, so a higher storey's tile occludes the storeys
/// beneath it. A framework layout const (the no-bare-types clause-4 plumbing carve-out).
/// Kept below [`GHOST_Z`] across all `MAX_LEVELS` storeys so the ghost stays the top
/// overlay.
pub(super) const STOREY_Z_GAP: f32 = 0.01;

/// The z-lift of a per-cell OVERLAY sprite (the stipple) above its own tile — a quarter
/// storey gap, so it draws in front of its tile yet stays inside its storey's z band
/// (below the next storey and far below [`GHOST_Z`]).
pub(super) const OVERLAY_Z_LIFT: f32 = STOREY_Z_GAP * 0.25;

/// The z-order of the hover ghost — above the base tiles so it reads as an overlay.
pub(super) const GHOST_Z: f32 = 1.0;

/// The hover ghost's tint when the placement is LEGAL — translucent white (a faint preview
/// over the cell). A framework layout const.
const GHOST_LEGAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.45);

/// The hover ghost's tint when the placement is ILLEGAL — translucent red (C4.4). A
/// framework layout const.
const GHOST_ILLEGAL: Color = Color::srgba(1.0, 0.2, 0.2, 0.55);

/// The EXISTS-BELOW class's base-tile tint (GTW-594 C2): a COOL BLUE-GREY hue at reduced
/// alpha, so a context storey's art reads categorically ghosted — never full-bright, never
/// mistakable for authored-here content. Composes with the stipple overlay
/// ([`STIPPLE_TINT`]).
pub(super) const GHOST_BELOW_TINT: Color = Color::srgba(0.55, 0.66, 0.82, 0.55);

/// The stipple overlay's tint (GTW-594 C2) — the 2×2-block checker laid over every
/// below-ghost tile, a deeper cool blue at low alpha so the pattern reads without hiding
/// the art beneath.
pub(super) const STIPPLE_TINT: Color = Color::srgba(0.35, 0.5, 0.75, 0.4);

/// The void grid's tint (GTW-594 C2) — the FAINT cell outline unpainted ACTIVE cells draw,
/// barely-there grey so empty cells read as "nothing authored here" without competing with
/// content.
pub(super) const VOID_GRID_TINT: Color = Color::srgba(0.65, 0.68, 0.75, 0.22);

/// The editor's treatment TABLE, base-tile column (GTW-594 C1/C2): the tint a storey's
/// BASE tile sprites render with, by [`StoreyTreatment`] class — or [`None`] for a storey
/// that draws nothing.
///
/// Only [`StoreyTreatment::Active`] maps to full-bright [`Color::WHITE`] (the A1 law's
/// editor half — pinned by the sibling `test` module); a
/// [`StoreyTreatment::ContextBelow`] storey is categorically ghosted
/// ([`GHOST_BELOW_TINT`]) regardless of depth (a flat, <=2-tier-trivially-clamped mapping
/// — the editor onions at most one storey by default anyway).
pub(super) const fn base_tile_tint(treatment: StoreyTreatment) -> Option<Color> {
    match treatment {
        StoreyTreatment::Hidden => None,
        StoreyTreatment::Active => Some(Color::WHITE),
        StoreyTreatment::ContextBelow(_) => Some(GHOST_BELOW_TINT),
    }
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
pub(super) fn draw_hover_ghost(
    commands: &mut Commands,
    atlas: &TileAtlas,
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    roles: &gdtf_battle_presenter::TileRoles,
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
pub(super) fn spawn_tile_sprite(
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
    spawn_preview_sprite(commands, sprite, cell, z);
}

/// Spawn one PATTERN overlay sprite (the stipple / the void grid — GTW-594 C2) at `cell`:
/// a plain image sprite over one of the generated [`overlay`](crate::preview::overlay)
/// textures, stretched to the cell and tinted (the patterns are authored white).
pub(super) fn spawn_overlay_sprite(
    commands: &mut Commands,
    image: Handle<bevy::image::Image>,
    cell: Cell,
    tint: Color,
    z: f32,
) {
    let mut sprite = Sprite::from_image(image);
    sprite.custom_size = Some(Vec2::splat(CELL_WORLD));
    sprite.color = tint;
    spawn_preview_sprite(commands, sprite, cell, z);
}

/// The shared preview-sprite spawn tail: position at the cell centre on the isolated
/// [`preview_layer`], marked [`PreviewTile`] so the change-driven redraw owns its lifetime.
fn spawn_preview_sprite(commands: &mut Commands, sprite: Sprite, cell: Cell, z: f32) {
    let world = cell_center_world(cell);
    commands.spawn((
        sprite,
        Transform::from_translation(world.extend(z)),
        preview_layer(),
        PreviewTile,
    ));
}
