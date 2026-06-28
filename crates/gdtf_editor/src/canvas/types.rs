//! Component types, layout constants, node builders, and tile-resolution helpers for the
//! map-editor canvas.
//!
//! The unit-marker types ([`CanvasRoot`], [`CanvasCell`], [`CanvasGhost`], [`CanvasScroll`]),
//! the rebuild-tracker ([`CanvasBuiltFor`]), the extent value-type ([`CanvasExtent`]), the three
//! layout constants ([`CANVAS_CELL_PX`], [`CELL_DASH_PX`], [`BOUNDARY_PX`]), the node-builder
//! functions ([`grid_container_node`], [`cell_node`], [`dimmed`]), the cell-spawner
//! ([`spawn_cell`]), and the shared tile-index resolver ([`paint_index_for`]) all live here.
//! They are co-located because every sibling module (sync, paint, ghost) depends on them and
//! none depends on the others — a single shared foundation with no cycles.

use bevy::{
    prelude::*,
    ui::{BorderColor, FlexDirection, FlexWrap, Val, widget::ImageNode},
};
use gdtf_battle_sim::{
    Cell,
    level::{
        GridHeight, GridSize, GridWidth, LevelTheme, ThemeCatalogRegistry, TileAtlasIndex, TileKey,
    },
};

use crate::{editor_map::EditorMap, session::MapEditorSession, tile_atlas::TileAtlas};

/// One drawable cell's square edge, in screen pixels — the chosen canvas SCALE (the GTW-423
/// open design decision).
///
/// A documented framework layout const (the no-bare-types clause-4 plumbing carve-out, the
/// `tile_atlas` `TERRAIN_TILE_PX` precedent), fed to the cell [`Node`] width/height. Chosen at
/// `24` so a cell reads comfortably yet the full `60 × 60` grid (1440 px) clearly overflows the
/// centre column — which the wrapping scroll list then scrolls. NOT a per-cell SIZE the sim
/// owns (the sim is render-free); a pure presentation magnitude.
pub(super) const CANVAS_CELL_PX: f32 = 24.0;

/// The per-cell grid-line border width, in screen pixels (C2) — the thin dimmed dash ruled
/// around each cell. A framework layout const (clause-4 plumbing carve-out).
pub(super) const CELL_DASH_PX: f32 = 1.0;

/// The drawable-area boundary border width, in screen pixels (C1) — the thicker dashed frame
/// around the whole grid. A framework layout const (clause-4 plumbing carve-out).
pub(super) const BOUNDARY_PX: f32 = 2.0;

/// Marker on the canvas's grid CONTAINER node — the single node that carries the C1 dashed
/// boundary border and holds every cell. A unit marker (no-bare-types); presence + its
/// [`CanvasExtent`] is the signal the rebuild + the test read.
///
/// It hangs inside the [`CanvasRegion`](crate::CanvasRegion)'s
/// [`ScrollListArea`](gdtf_ui::ScrollListArea) (the GTW-421 parenting rule). The rebuild despawns
/// it (subtree) and respawns it on a size/theme change.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasRoot;

/// The grid extent the [`CanvasRoot`] was built for — its `width × height` cell counts (C1),
/// carried on the root so the boundary's extent is readable without re-deriving it.
///
/// A named newtype-bearing component (no-bare-types): the two spans are the sim's own
/// [`GridWidth`] / [`GridHeight`] newtypes (private inner, derived `Deref` to `u8`), so a test
/// can assert the boundary is sized from `grid_size` with a same-type comparison — no lossy
/// `u16` widening. Z (levels) is excluded — 2D x/y only (see module).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasExtent {
    /// The grid's x span in cells (the [`GridSize::width`] the canvas drew).
    pub(super) width:  GridWidth,
    /// The grid's y span in cells (the [`GridSize::height`] the canvas drew).
    pub(super) height: GridHeight,
}

impl CanvasExtent {
    /// Build a canvas extent from a [`GridSize`]'s x/y spans (z is out of scope — 2D plane).
    #[must_use]
    pub(super) const fn from_grid(size: GridSize) -> Self {
        Self {
            width:  size.width(),
            height: size.height(),
        }
    }

    /// The grid's x span in cells.
    #[must_use]
    pub const fn width(&self) -> GridWidth {
        self.width
    }

    /// The grid's y span in cells.
    #[must_use]
    pub const fn height(&self) -> GridHeight {
        self.height
    }

    /// The total cell count this extent draws (`width × height`) — the number of cell fills the
    /// canvas spawns (C3) and the count the test asserts.
    ///
    /// Not `const` because [`Deref`] is not const-callable on stable — the inner `u8` is reached
    /// at runtime only.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        usize::from(*self.width) * usize::from(*self.height)
    }
}

/// Marker on one drawable CELL — a pre-filled [`ImageNode`] carrying the theme's default-floor
/// tile sprite (C3) and its own per-cell dimmed-dash border (C2). The test counts these to
/// assert one fill per `width × height` cell.
///
/// Carries the cell's ground-plane [`Cell`] coordinate (GTW-426): the canvas spawns cells in
/// row-major order, so cell index `i` is `(x = i % width, y = i / width)`. Storing the [`Cell`]
/// (the sim's coordinate newtype — no-bare-types, NOT a bare index) lets the hover-ghost and the
/// click-to-paint flow speak the same coordinate vocabulary as the [`EditorMap`] model. A
/// `Button` is `#[require]`d on the cell so the engine's `ui_focus_system` drives its
/// [`Interaction`] (hover + click) under `DefaultPlugins` (bevy-traps #6).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasCell {
    /// The cell's ground-plane coordinate (`x` in `0..width`, `y` in `0..height`). The key the
    /// hover-ghost snaps to and the click-to-paint flow writes into the [`EditorMap`].
    cell: Cell,
}

impl CanvasCell {
    /// Build a canvas-cell marker for a ground-plane [`Cell`] coordinate.
    #[must_use]
    pub(super) const fn new(cell: Cell) -> Self {
        Self { cell }
    }

    /// The cell's ground-plane coordinate — the key the ghost snaps to and the paint flow writes.
    #[must_use]
    pub const fn cell(&self) -> Cell {
        self.cell
    }
}

/// Marker on the single overlay **hover ghost** — a translucent preview [`ImageNode`] of the
/// selected tile that snaps to the hovered cell (GTW-426 C1).
///
/// A unit marker (no-bare-types). ONE persistent ghost entity (the ui-mutate rule: it is
/// re-parented + retinted in place, never respawned per hover). It is re-parented under the
/// hovered [`CanvasCell`] (so it tracks that cell's layout for free — the snap) and shown only
/// while a [`CanvasCell`] is [`Interaction::Hovered`] AND a tile is selected; otherwise hidden.
/// Its reduced-alpha tint reads it as a PREVIEW, distinct from a painted cell.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasGhost;

/// The grid + theme a [`CanvasRoot`] was last built for — the [`sync_canvas`](super::sync::sync_canvas)
/// rebuild tracker.
///
/// A named [`Local`]-state newtype (no-bare-types): pairs the [`GridSize`] and [`LevelTheme`]
/// the canvas last drew, so the rebuild fires on EITHER a size change (new extent / cell count)
/// or a theme change (new default-floor fill) — C4 — and on neither for an unrelated session
/// mutation. `None` before the first build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CanvasBuiltFor {
    /// The grid size the canvas was last built for.
    pub(crate) size:  GridSize,
    /// The theme whose default floor the cells were last filled with.
    pub(crate) theme: LevelTheme,
}

/// Marker on the canvas's wrapping scroll-list ROOT FRAME (the `gdtf_ui` scroll list put up by
/// [`spawn_canvas_scroll`](super::scroll::spawn_canvas_scroll)). A unit marker (no-bare-types)
/// so a test can find the scroll list.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasScroll;

/// The grid CONTAINER [`Node`] (C1/C2): a flex-WRAP row sized to exactly `width` cells across so
/// the cells wrap into `height` rows, carrying the thick C1 boundary border. Fixed-px sizing
/// (the chosen scale model) — the only place the editor uses `Px`, because the canvas is a
/// pixel-true cell grid that scrolls, not a responsive panel.
pub(super) fn grid_container_node(extent: CanvasExtent) -> Node {
    // Width = exactly `width` cells of [`CANVAS_CELL_PX`] each, so `flex_wrap` breaks the run
    // into exactly `height` rows of exactly `width` cells.
    //
    // BOX-MODEL NOTE: Bevy's default `BoxSizing` is `BorderBox`, so the 1 px per-cell dash
    // border (C2) paints INSIDE the 24 px cell footprint — each cell's real outer edge is still
    // 24 px (= `CANVAS_CELL_PX`), NOT 24 + 2×1 = 26 px. The old formula used
    // `cell_outer = CANVAS_CELL_PX + 2 * CELL_DASH_PX = 26 px`, which inflated the container
    // width so `flex_wrap` packed `floor(26 * W / 24)` cells per row instead of exactly `W` —
    // at W = 60 that is 65, causing right-overflow and a ragged short bottom-right. The fix:
    // `cell_outer = CANVAS_CELL_PX` (24 px), the actual BorderBox footprint. The dash still
    // draws inside each cell's 24 px — visually unchanged — and row math is now exact.
    let cell_outer = CANVAS_CELL_PX;
    let row_width = cell_outer * f32::from(*extent.width());
    Node {
        width: Val::Px(2.0f32.mul_add(BOUNDARY_PX, row_width)),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        border: UiRect::all(Val::Px(BOUNDARY_PX)),
        ..default()
    }
}

/// One cell's [`Node`] (C2/C3): a fixed [`CANVAS_CELL_PX`] square with a thin per-cell dash
/// border. The fill [`ImageNode`] (C3) draws over the node's content box.
pub(super) fn cell_node() -> Node {
    Node {
        width: Val::Px(CANVAS_CELL_PX),
        height: Val::Px(CANVAS_CELL_PX),
        border: UiRect::all(Val::Px(CELL_DASH_PX)),
        ..default()
    }
}

/// Dim a theme color toward transparency for the per-cell grid-line dash (C2) — the per-cell
/// rules read as faint guides, distinct from the thicker opaque C1 boundary. Halves the alpha.
pub(super) fn dimmed(color: Color) -> Color {
    let rgba = color.to_srgba();
    Color::srgba(rgba.red, rgba.green, rgba.blue, rgba.alpha * 0.5)
}

/// Resolve a PAINTED tile's atlas index — the index a painted cell's fill sprite shows (C2).
///
/// Looks the painted [`TileKey`] up in the active `theme`'s catalog and returns its
/// [`CatalogTile`](gdtf_battle_sim::level::CatalogTile)'s
/// [`TileAtlasIndex`](gdtf_battle_sim::level::TileAtlasIndex), or [`None`] if the theme has no
/// catalog / the key names no tile (a stale paint after a theme switch — the cell then falls back
/// to the default-floor fill rather than panicking). Shared by sync, paint, and ghost.
pub(super) fn paint_index_for(
    registry: &ThemeCatalogRegistry,
    theme: LevelTheme,
    key: &TileKey,
) -> Option<TileAtlasIndex> {
    registry
        .catalog(theme)
        .and_then(|catalog| catalog.tile(key))
        .map(|tile| tile.atlas_index)
}

/// Spawn one drawable cell at ground-plane coordinate `cell`: a fixed-[`CANVAS_CELL_PX`]
/// clickable [`Button`] [`ImageNode`] pre-filled with the default-floor tile sprite (C3) and
/// ruled with a per-cell dimmed-dash border (C2).
///
/// A [`Button`] so the engine's `ui_focus_system` drives its [`Interaction`] (hover for the
/// ghost — C1, press for the paint — C2) under `DefaultPlugins` (bevy-traps #6). When the default
/// floor resolves, the cell is an [`ImageNode::from_atlas_image`] over the terrain sheet at the
/// tile's atlas index (the palette-row precedent); when it does not (a theme with no catalog) the
/// cell is a plain bordered box — boundary + dashes still render.
pub(super) fn spawn_cell(
    commands: &mut Commands,
    atlas: &TileAtlas,
    fill_index: Option<TileAtlasIndex>,
    dash_color: Color,
    cell: Cell,
) -> Entity {
    let mut entity = commands.spawn((
        CanvasCell::new(cell),
        Button,
        cell_node(),
        BorderColor::all(dash_color),
    ));
    if let Some(index) = fill_index {
        entity.insert(ImageNode::from_atlas_image(
            atlas.image(),
            TextureAtlas {
                layout: atlas.layout(),
                index:  *index,
            },
        ));
    }
    entity.id()
}

/// Build the canvas: the [`CanvasRoot`] grid container (carrying the C1 dashed boundary + the
/// [`CanvasExtent`]) holding one [`CanvasCell`] per `width × height` cell, each filled with its
/// PAINTED tile from the [`EditorMap`] model where painted (C2) and the theme default-floor sprite
/// otherwise (C3), and bordered with a per-cell dimmed dash (C2). The root is deferred-parented
/// under the [`CanvasRegion`](crate::CanvasRegion)'s [`ScrollListArea`] (the GTW-421 parenting rule).
#[expect(
    clippy::too_many_arguments,
    reason = "the canvas build threads theme + atlas + the two catalog sources (registry/session) \
              + the paint model + extent + the resolved default-floor index; each is a distinct \
              concern and collapsing them into a struct would not reduce coupling"
)]
pub(super) fn spawn_canvas(
    commands: &mut Commands,
    theme: &gdtf_ui::theme::GdtfTheme,
    atlas: &TileAtlas,
    registry: &ThemeCatalogRegistry,
    session: &MapEditorSession,
    map: &EditorMap,
    extent: CanvasExtent,
    fill_index: Option<TileAtlasIndex>,
) {
    let boundary_color = *theme.panel.border_color;
    let dash_color = dimmed(*theme.panel.border_color);
    let width = i32::from(*extent.width());
    let cells: Vec<Entity> = (0..extent.cell_count())
        .map(|index| {
            // Row-major: cell `index` is the ground-plane cell `(x = index % width, y = index /
            // width)`. The grid container is a `flex_wrap` row of exactly `width` cells, so the
            // spawn order matches the on-screen layout order — the cell carries this coordinate so
            // the hover-ghost + paint flow can resolve it without re-deriving from layout.
            #[expect(
                clippy::cast_possible_wrap,
                clippy::cast_possible_truncation,
                reason = "index < cell_count = width*height <= 60*60 = 3600, well within i32"
            )]
            let i = index as i32;
            let cell = Cell::new(i % width, i / width);
            // A painted cell (in the model) shows its painted tile's atlas index (C2 persistence
            // across rebuild); an unpainted cell falls back to the theme default-floor (C3).
            let cell_index = map
                .tile_at(cell)
                .and_then(|key| paint_index_for(registry, session.theme(), key))
                .or(fill_index);
            spawn_cell(commands, atlas, cell_index, dash_color, cell)
        })
        .collect();
    let root = commands
        .spawn((
            CanvasRoot,
            extent,
            grid_container_node(extent),
            BorderColor::all(boundary_color),
            BackgroundColor(*theme.panel.color),
        ))
        .add_children(&cells)
        .id();

    commands.queue(move |world: &mut World| {
        let Some(area) = canvas_scroll_area(world) else {
            return;
        };
        if let Ok(mut area_entity) = world.get_entity_mut(area) {
            area_entity.add_child(root);
        }
    });
}

/// Find the [`CanvasRegion`](crate::CanvasRegion)'s [`ScrollListArea`] — the clipping, scrolling
/// viewport child of the region's scroll-list grid root frame (the GTW-421 parenting rule).
///
/// The [`CanvasRegion`] marker rides the [`CanvasRegion`] panel; that panel holds the scroll
/// list whose [`ScrollListArea`] is the canvas's scroll viewport. Walks the panel's descendants
/// for the area. Returns [`None`] if the area is not yet present (the deferred command runs
/// after the shell spawn applied the scroll list, so it exists by then).
fn canvas_scroll_area(world: &mut World) -> Option<Entity> {
    let region = world
        .query_filtered::<Entity, With<crate::CanvasRegion>>()
        .iter(world)
        .next()?;
    find_scroll_area(world, region)
}

/// Walk a subtree (depth-first from `root`) for the first [`ScrollListArea`] descendant — the
/// canvas scroll viewport sits a couple of levels under the [`CanvasRegion`] panel (the panel
/// holds a wrapper cell that holds the scroll-list frame whose area child is the viewport).
fn find_scroll_area(world: &World, root: Entity) -> Option<Entity> {
    if world
        .get_entity(root)
        .is_ok_and(|entity| entity.contains::<gdtf_ui::ScrollListArea>())
    {
        return Some(root);
    }
    let children = world.get::<Children>(root)?;
    children
        .iter()
        .find_map(|child| find_scroll_area(world, child))
}
