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
//! [`CurrentEditLevel`] storey, the [`HoveredCell`], or the [`ViewMode`] (GTW-532: the full-view
//! toggle widens the drawn storey band) — so a static frame costs nothing. It reads
//! the SAME model + resolution the save path + the capture drive read (the theme palette, the
//! per-def atlas index resolved THE WAY THE PRESENTER DOES via [`terrain_atlas_index`], the shared
//! [`evaluate_placement`] legality), so the preview shows exactly what a save would write.

use core::ops::RangeInclusive;

use bevy::{image::TextureAtlas, prelude::*};
use gdtf_battle_presenter::{TileRoles, ViewMode};
use gdtf_battle_sim::{
    Cell,
    level::{GridSize, UuidThemeRegistry},
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

/// The z-order of a base (floor / painted) tile sprite on the GROUND storey — below the hover
/// ghost. Higher storeys are lifted by [`STOREY_Z_GAP`] per storey so an upper floor draws in
/// FRONT of a lower one (the painter's-algorithm occlusion the presenter's per-storey Z gives).
const TILE_Z: f32 = 0.0;

/// The per-storey z-lift for the [`ViewMode::FullView`] stack (GTW-532) — each storey above the
/// ground is drawn `STOREY_Z_GAP` further forward, so a higher storey's tile occludes the storeys
/// beneath it (the painter's-algorithm rule the presenter's per-storey Z gives). A framework layout
/// const (the no-bare-types clause-4 plumbing carve-out), not a domain value. Kept below
/// [`GHOST_Z`] across all `MAX_LEVELS` storeys so the ghost still reads as the top overlay.
const STOREY_Z_GAP: f32 = 0.01;

/// The z-order of the hover ghost — above the base tiles so it reads as an overlay.
const GHOST_Z: f32 = 1.0;

/// The hover ghost's tint when the placement is LEGAL — translucent white (a faint preview over
/// the cell). A framework layout const.
const GHOST_LEGAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.45);

/// The hover ghost's tint when the placement is ILLEGAL — translucent red (C4.4). A framework
/// layout const.
const GHOST_ILLEGAL: Color = Color::srgba(1.0, 0.2, 0.2, 0.55);

/// The inclusive storey RANGE the prefab preview draws, chosen by the [`ViewMode`] (GTW-532 C1 /
/// C2) — the editor-side mirror of the presenter's
/// [`drawn_band`](gdtf_battle_presenter::ViewMode) logic, reusing the SAME [`ViewMode`] type (NOT a
/// parallel toggle enum):
///
/// - [`ViewMode::DownToActive`] (default) → `0..=active`: draw the ground floor up to and INCLUDING
///   the current edit storey and cull everything above it — the GTW-515 behaviour, unchanged.
/// - [`ViewMode::FullView`] → `0..=(levels - 1)`: draw the WHOLE storey stack of the prefab's own
///   volume regardless of the edit level (the full-view toggle). The ceiling is the prefab's own
///   level count minus one (never past [`MAX_LEVELS`] — [`GridLevels`] clamps that on construction),
///   so it draws exactly the prefab's storeys and no empty ones.
///
/// Pure (no Bevy borrows) so the mode-switch → drawn-range change is unit-tested on the real path
/// (C4). `active` reads through [`CurrentEditLevel`]'s `Deref`; the range is over `u8` storey
/// indices.
///
/// [`GridLevels`]: gdtf_battle_sim::level::GridLevels
/// [`MAX_LEVELS`]: gdtf_battle_sim::metric::MAX_LEVELS
#[must_use]
pub(crate) fn drawn_storeys(active: Level, view: ViewMode, size: GridSize) -> RangeInclusive<u8> {
    let ceiling = match view {
        // DEFAULT (GTW-515): cull above the active edit storey.
        ViewMode::DownToActive => *active,
        // FULL VIEW (GTW-532): the prefab's top valid storey. `levels >= 1` (GridSize rejects an
        // empty grid), so `saturating_sub(1)` is the last storey index; the whole stack is drawn.
        ViewMode::FullView => size.levels().saturating_sub(1),
    };
    0..=ceiling
}

/// `Update` (in `Editing`): redraw the preview tiles when any input the preview depends on changed
/// (GTW-515 C4.3 / C4.4; GTW-532: the [`ViewMode`] full-view toggle widens the drawn storey range).
///
/// Despawns every existing [`PreviewTile`] and respawns a sprite per drawable cell of every storey
/// in the [`ViewMode`]-chosen band ([`drawn_storeys`]): in [`ViewMode::DownToActive`] that band is
/// the single edit storey (`0..=active`, the GTW-515 behaviour); in [`ViewMode::FullView`] it is the
/// whole prefab stack (`0..=levels-1`), each storey lifted a per-storey z so an upper floor draws in
/// front. The active/ground storey's UNPAINTED cells draw the theme default-floor; a PAINTED cell
/// (any storey) draws its own tile. Then, if a cell is hovered, draws the translucent HOVER GHOST
/// over it — RED when [`evaluate_placement`] rejects the (hovered cell, selected tile) placement,
/// faint-white when legal. Change-driven (guarded on `is_changed` of every input — INCLUDING the
/// [`ViewMode`], so the toggle re-runs the draw) so a static frame does no work.
///
/// All model borrows are `Option` (state-scoped — bevy-traps #1); no-ops until they + the
/// registries resolve.
#[expect(
    clippy::too_many_arguments,
    reason = "the preview redraw reads every model input it depends on (map, session, edit level, \
              hover, view mode) + the three shared registries (terrain defs, themes, tile roles) + \
              the tile atlas + Commands to (re)spawn; each is a distinct Bevy SystemParam and Bevy's \
              injection cannot reduce them without a wrapper resource that changes the crate API"
)]
pub(crate) fn redraw_preview_tiles(
    mut commands: Commands,
    map: Option<Res<EditorMap>>,
    session: Option<Res<MapEditorSession>>,
    edit_level: Option<Res<CurrentEditLevel>>,
    hovered: Option<Res<HoveredCell>>,
    view: Option<Res<ViewMode>>,
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
        Some(view),
        Some(registry),
        Some(themes),
        Some(roles),
        Some(atlas),
    ) = (
        map, session, edit_level, hovered, view, registry, themes, roles, atlas,
    )
    else {
        return;
    };

    // CHANGE-DRIVEN: only redraw when an input the preview depends on changed (or the atlas just
    // loaded / registries just resolved — is_changed covers first-insert too). GTW-532: the
    // ViewMode toggle re-runs the draw exactly the way the level-nav already does (`edit_level`).
    let dirty = map.is_changed()
        || session.is_changed()
        || edit_level.is_changed()
        || hovered.is_changed()
        || view.is_changed()
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

    // BASE layer: one sprite per drawable cell of EVERY storey in the ViewMode-chosen band (GTW-532
    // C1 / C2) — painted tile if any, else the theme default-floor. In DownToActive that band is the
    // single edit storey (`0..=active`, unchanged GTW-515); in FullView it is the whole prefab stack
    // (`0..=levels-1`), each storey lifted a per-storey z (STOREY_Z_GAP) so an upper floor draws in
    // FRONT (the presenter's painter's-algorithm occlusion). An unresolved tile (no atlas index)
    // draws nothing (the clear colour shows through), matching the presenter's fall-back behaviour.
    for storey in drawn_storeys(level, *view, size) {
        let storey_level = Level::new(storey);
        let z = STOREY_Z_GAP.mul_add(f32::from(storey), TILE_Z);
        for y in 0..height {
            for x in 0..width {
                let cell = Cell::new(x, y);
                let slot = CellLevel::new(cell, storey_level);
                // In FullView, only PAINTED cells of an upper storey are drawn (an unpainted upper
                // cell is empty air, not a floor — a full default-floor fill on every storey would
                // opaquely hide the whole stack below). The active/ground storey keeps its
                // default-floor fill so the base plane always reads.
                let tile = if storey == *level {
                    map.tile_at_level(slot).or(default_floor)
                } else {
                    map.tile_at_level(slot)
                };
                let Some(tile) = tile else {
                    continue;
                };
                let Some(index) = terrain_atlas_index(&registry, &roles, &tile) else {
                    continue;
                };
                spawn_tile_sprite(&mut commands, &atlas, cell, *index, Color::WHITE, z);
            }
        }
    }

    // HOVER GHOST: a translucent overlay on the hovered cell (the active edit storey), RED when the
    // placement is illegal — always the top overlay (GHOST_Z above every storey band).
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

#[cfg(test)]
mod tests {
    use gdtf_battle_presenter::ViewMode;
    use gdtf_battle_sim::{
        level::{GridHeight, GridLevels, GridSize, GridWidth},
        metric::{Level, MAX_LEVELS},
    };

    use super::drawn_storeys;

    /// A `4 × 4 × 4` volume (valid storeys 0, 1, 2, 3), or a `1×1×1` fallback (fallible ctor,
    /// panic-free per the workspace lints).
    fn size(levels: u8) -> GridSize {
        GridSize::new(
            GridWidth::new(4),
            GridHeight::new(4),
            GridLevels::new(levels),
        )
        .unwrap_or_else(|_| GridSize::default())
    }

    /// C1 / C2 — [`ViewMode::DownToActive`] (the default) draws `0..=active`: exactly the
    /// GTW-515 single-storey-and-below band, unchanged. The `FullView` ceiling never bleeds in.
    #[test]
    fn down_to_active_draws_up_to_the_edit_storey() {
        let size = size(4); // storeys 0..=3
        assert_eq!(
            drawn_storeys(Level::new(0), ViewMode::DownToActive, size),
            0..=0,
            "at the ground storey DownToActive draws only storey 0",
        );
        assert_eq!(
            drawn_storeys(Level::new(2), ViewMode::DownToActive, size),
            0..=2,
            "at storey 2 DownToActive draws 0..=2 (up to and including the active storey)",
        );
    }

    /// C1 / C2 — [`ViewMode::FullView`] draws the WHOLE prefab stack `0..=(levels-1)` regardless of
    /// the active edit storey: the drawn range is STRICTLY WIDER than `DownToActive` at the same edit
    /// level (this is the toggle's observable effect — the mode switch changes the drawn range).
    #[test]
    fn full_view_draws_the_whole_prefab_stack() {
        let size = size(4); // storeys 0..=3
        let active = Level::new(1);
        let down = drawn_storeys(active, ViewMode::DownToActive, size);
        let full = drawn_storeys(active, ViewMode::FullView, size);
        assert_eq!(down, 0..=1, "DownToActive at storey 1 draws 0..=1");
        assert_eq!(
            full,
            0..=3,
            "FullView draws the whole 4-storey stack (0..=levels-1) regardless of the edit storey",
        );
        assert!(
            full.count() > down.count(),
            "the FullView band is strictly wider than DownToActive at the same edit storey — the \
             toggle changes the drawn storey range (C2)",
        );
    }

    /// C2 — the `FullView` ceiling is the PREFAB's OWN level count minus one, not a fixed
    /// [`MAX_LEVELS`]: a single-storey prefab draws only storey 0 even in `FullView` (never empty
    /// storeys above its volume).
    #[test]
    fn full_view_ceiling_is_the_prefab_level_count() {
        assert_eq!(
            drawn_storeys(Level::new(0), ViewMode::FullView, size(1)),
            0..=0,
            "a 1-storey prefab in FullView draws only storey 0 (its own volume, not MAX_LEVELS)",
        );
        assert_eq!(
            drawn_storeys(Level::new(0), ViewMode::FullView, size(MAX_LEVELS)),
            0..=(MAX_LEVELS - 1),
            "a full-height prefab in FullView draws every storey up to MAX_LEVELS-1",
        );
    }
}
