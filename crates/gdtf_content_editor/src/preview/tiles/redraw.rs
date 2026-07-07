//! The change-driven preview REDRAW (GTW-515 C4.3 / C4.4, GTW-594 C2) — despawn-all +
//! respawn of the per-cell tile sprites, classified per storey through the presenter's ONE
//! shared [`storey_treatment`] classifier and rendered through the editor's treatment
//! table (`sprites.rs`).

use bevy::prelude::*;
use gdtf_battle_presenter::{
    ActiveLevel, IsolateView, StoreyTreatment, StoreyViewMode, ViewMode, storey_treatment,
};
use gdtf_battle_sim::{
    level::{GridSize, UuidThemeRegistry},
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::sprites::{
    MISSING_SPRITE_TINT, OVERLAY_Z_LIFT, STIPPLE_TINT, STOREY_Z_GAP, TILE_Z, VOID_GRID_TINT,
    base_tile_tint, draw_hover_ghost, spawn_color_tile, spawn_overlay_sprite, spawn_tile_sprite,
};
use crate::{
    canvas::CurrentEditLevel,
    editor_map::EditorMap,
    hovered_cell::HoveredCell,
    preview::{overlay::PreviewOverlayImages, target::PreviewTile},
    session::MapEditorSession,
    terrain_graphics::terrain_sprite_def,
};

/// The GROUND storey index (z=0) — the base plane that carries the theme default-floor fill
/// on its unpainted cells (GTW-535) WHENEVER it is drawn. A framework layout const (the loop
/// iterates bare `u8` storey indices), not a domain value.
const GROUND_STOREY: u8 = 0;

/// `Update` (in `Editing`): redraw the preview tiles when any input the preview depends on
/// changed (GTW-515 C4.3 / C4.4; GTW-532 the [`ViewMode`] toggle; GTW-594 the
/// [`IsolateView`] toggle).
///
/// Despawns every existing [`PreviewTile`] and respawns the viewport per storey, each
/// storey classified through the presenter's ONE shared [`storey_treatment`] classifier
/// (GTW-594 C1 — the editor CONSUMES the classifier, never a parallel band rule) over the
/// prefab's own `0..levels` volume, with the edit storey as the active level. Per cell the
/// editor's treatment table (GTW-594 C2) renders three unmistakable classes:
///
/// - **authored-here** — a cell with content on the [`StoreyTreatment::Active`] storey
///   draws its tile FULL-BRIGHT (only Active may — the A1 law);
/// - **exists-below** — a cell with content on a [`StoreyTreatment::ContextBelow`] storey
///   draws its tile in the cool blue-grey hue+alpha ghost tint PLUS the 2×2-block stipple
///   overlay;
/// - **empty** — an unpainted cell on the ACTIVE storey draws the faint void grid (nothing
///   authored here); unpainted context cells draw nothing.
///
/// Only the GROUND storey's unpainted cells carry the theme default-floor (the base plane —
/// GTW-535), and only when the ground storey is IN the drawn band: in the editor's DEFAULT
/// Isolate view (band floor = active) a higher edit storey therefore shows ONLY its
/// authored content plus the categorical ghost below — the GTW-592 repro fixed. A
/// [`StoreyTreatment::Hidden`] storey draws nothing at all. Then, if a cell is hovered,
/// the translucent HOVER GHOST draws over it — RED when illegal (C4.4). Change-driven
/// (guarded on `is_changed` of every input — INCLUDING the [`ViewMode`] and the GTW-594
/// [`IsolateView`], so both toggles re-run the draw) so a static frame does no work.
///
/// All model borrows are `Option` (state-scoped — bevy-traps #1); no-ops until they + the
/// registries resolve.
#[expect(
    clippy::too_many_arguments,
    reason = "the preview redraw reads every model input it depends on (map, session, edit level, \
              hover, view mode, isolate toggle) + the three shared registries (terrain defs, \
              themes, sprite defs) + the asset server + the generated overlay textures + Commands \
              to (re)spawn; each is a distinct Bevy SystemParam and Bevy's injection cannot reduce \
              them without a wrapper resource that changes the crate API"
)]
pub(crate) fn redraw_preview_tiles(
    mut commands: Commands,
    map: Option<Res<EditorMap>>,
    session: Option<Res<MapEditorSession>>,
    edit_level: Option<Res<CurrentEditLevel>>,
    hovered: Option<Res<HoveredCell>>,
    view: Option<Res<ViewMode>>,
    isolate: Option<Res<IsolateView>>,
    overlays: Option<Res<PreviewOverlayImages>>,
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    sprites: Option<Res<SpriteDefRegistry>>,
    asset_server: Option<Res<AssetServer>>,
    existing: Query<Entity, With<PreviewTile>>,
) {
    let (
        Some(map),
        Some(session),
        Some(edit_level),
        Some(hovered),
        Some(view),
        Some(isolate),
        Some(overlays),
        Some(registry),
        Some(themes),
        Some(sprites),
        Some(asset_server),
    ) = (
        map,
        session,
        edit_level,
        hovered,
        view,
        isolate,
        overlays,
        registry,
        themes,
        sprites,
        asset_server,
    )
    else {
        return;
    };

    // CHANGE-DRIVEN: only redraw when an input the preview depends on changed (or the
    // overlays just loaded / registries just resolved — is_changed covers first-insert too).
    // GTW-532/GTW-594: the ViewMode + IsolateView toggles re-run the draw exactly the way the
    // level-nav already does (`edit_level`).
    let dirty = map.is_changed()
        || session.is_changed()
        || edit_level.is_changed()
        || hovered.is_changed()
        || view.is_changed()
        || isolate.is_changed()
        || overlays.is_changed()
        || registry.is_changed()
        || themes.is_changed()
        || sprites.is_changed();
    if !dirty {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let level = edit_level.level();
    let size = session.grid_size();
    let theme = session.theme();
    let pass = StoreyPass {
        map: &map,
        registry: &registry,
        sprites: &sprites,
        asset_server: &asset_server,
        overlays: &overlays,
        default_floor: session
            .default_floor()
            .or_else(|| themes.default_floor(&theme)),
        size,
    };

    // GTW-594 C1: classify each storey of the prefab's own volume through the presenter's
    // ONE shared classifier — the edit storey is the active level; the composed mode pairs
    // the two-state ViewMode with the Isolate toggle (Isolate wins, decided IN the
    // classifier).
    let active = ActiveLevel::new(level);
    let mode = StoreyViewMode::new(*view, *isolate);
    for storey in 0..*size.levels() {
        let treatment = storey_treatment(Level::new(storey), active, mode);
        draw_storey(&mut commands, &pass, storey, treatment);
    }

    // HOVER GHOST: a translucent overlay on the hovered cell (the active edit storey), RED when the
    // placement is illegal — always the top overlay (GHOST_Z above every storey band).
    draw_hover_ghost(
        &mut commands,
        pass.asset_server,
        &map,
        &registry,
        &sprites,
        &session,
        &hovered,
        level,
    );
}

/// The borrowed model + asset inputs one storey's per-cell pass reads — a plumbing
/// aggregate of borrows (the `ViewportCtx` pattern), so [`draw_storey`] stays under the
/// argument/line lints without obscuring its inputs. Not a domain value.
struct StoreyPass<'a> {
    /// The paintable map (read for each cell's painted tile).
    map:           &'a EditorMap,
    /// The terrain registry (the per-def graphic-name resolve).
    registry:      &'a TerrainDefRegistry,
    /// The GTW-663 sprite-def registry (the same def-driven resolve the presenter uses —
    /// GTW-665).
    sprites:       &'a SpriteDefRegistry,
    /// Loads each def's source image by its authored path (the same handle the
    /// battle renderer holds for the same path).
    asset_server:  &'a AssetServer,
    /// The GTW-594 generated stipple / void-grid overlay sheets.
    overlays:      &'a PreviewOverlayImages,
    /// The theme default-floor the GROUND storey's unpainted cells fall back to (GTW-535).
    default_floor: Option<TerrainUuid>,
    /// The prefab's grid volume (the cell loop bounds).
    size:          GridSize,
}

/// Draw ONE storey's cells under its [`StoreyTreatment`] (GTW-594 C2) — the editor's
/// treatment table applied per cell: full-bright tiles on the Active storey (plus the
/// faint void grid on its unpainted cells), ghost-tinted tiles PLUS the stipple overlay on
/// a context storey, nothing on a Hidden one.
fn draw_storey(
    commands: &mut Commands,
    pass: &StoreyPass<'_>,
    storey: u8,
    treatment: StoreyTreatment,
) {
    // The editor's treatment table: Hidden draws nothing; Active is the one full-bright
    // storey; ContextBelow is the categorical ghost.
    let Some(tint) = base_tile_tint(treatment) else {
        return;
    };
    let is_active_storey = treatment == StoreyTreatment::Active;
    let is_context_storey = matches!(treatment, StoreyTreatment::ContextBelow(_));
    let storey_level = Level::new(storey);
    let z = STOREY_Z_GAP.mul_add(f32::from(storey), TILE_Z);
    for y in 0..i32::from(*pass.size.height()) {
        for x in 0..i32::from(*pass.size.width()) {
            let cell = Cell::new(x, y);
            let slot = CellLevel::new(cell, storey_level);
            // Only the GROUND storey (z=0 / storey 0) gets the theme default-floor fill
            // on its UNPAINTED cells — the base plane (GTW-535) — and only when the
            // classifier DRAWS the ground storey at all (in the Isolate view a higher
            // edit storey's band excludes it — the GTW-592 fix). UPPER storeys draw
            // ONLY painted cells (an unpainted upper cell is empty air, not a floor).
            let tile = if storey == GROUND_STOREY {
                pass.map.tile_at_level(slot).or(pass.default_floor)
            } else {
                pass.map.tile_at_level(slot)
            };
            let Some(tile) = tile else {
                // EMPTY on the ACTIVE storey → the faint void grid (the third
                // categorical class); empty context cells draw nothing.
                if is_active_storey {
                    spawn_overlay_sprite(
                        commands,
                        pass.overlays.void_grid(),
                        cell,
                        VOID_GRID_TINT,
                        z,
                    );
                }
                continue;
            };
            // GTW-665: resolve the painted tile's sprite def THE WAY THE PRESENTER DOES;
            // a graphic that resolves NO def draws the LOUD magenta missing quad (C4 —
            // matching the battle renderer's missing-marker semantics, never invisible).
            match terrain_sprite_def(pass.registry, pass.sprites, &tile) {
                Some(def) => {
                    spawn_tile_sprite(commands, pass.asset_server, cell, def, tint, z);
                }
                None => spawn_color_tile(commands, cell, MISSING_SPRITE_TINT, z),
            }
            // EXISTS-BELOW: the stipple overlay rides every context tile (GTW-594 C2 —
            // hue+alpha tint PLUS the pattern, so the class reads at any zoom; the
            // blue-grey base tint itself came through `base_tile_tint` above).
            if is_context_storey {
                spawn_overlay_sprite(
                    commands,
                    pass.overlays.stipple(),
                    cell,
                    STIPPLE_TINT,
                    z + OVERLAY_Z_LIFT,
                );
            }
        }
    }
}
