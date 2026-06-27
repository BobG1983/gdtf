//! The map-editor **central canvas** — the drawable cell grid that fills the GTW-417
//! [`CanvasRegion`](crate::CanvasRegion) (GTW-423).
//!
//! Once a size is set (the GTW-421 [`MapEditorSession::grid_size`]), the canvas draws the
//! drawable area as a `width × height` grid of cells, each PRE-FILLED with the current
//! theme's default-floor tile SPRITE (C3), bounded by a DASHED boundary (C1) and ruled with
//! PER-CELL dimmed dashes (C2). It LIVE-updates on theme / size change (C4): a theme switch
//! repaints every cell with the new default floor, a size change re-extents the grid.
//!
//! ## Scope: 2D x/y ground plane only
//!
//! The canvas draws ONLY the x/y ground plane — the grid's [`width`](GridSize::width) ×
//! [`height`](GridSize::height). The z axis ([`levels`](GridSize::levels)) is OUT of scope:
//! no level (storey) SELECTION control exists yet, so there is no way to choose which storey
//! to draw. When level selection lands the canvas will draw the chosen storey's x/y plane;
//! until then it draws the single ground plane.
//!
//! ## Chosen scale model (the GTW-423 open design decision)
//!
//! Each cell is rendered as a UI [`ImageNode`] of fixed edge [`CANVAS_CELL_PX`] inside the
//! [`CanvasRegion`]. Because the full `60 × 60` grid (1440 px at [`CANVAS_CELL_PX`]) far
//! exceeds the centre column, the canvas grid is WRAPPED in a `gdtf_ui`
//! [`spawn_scroll_list`](gdtf_ui::spawn_scroll_list) — so it SCROLLS rather than squeezing.
//! The grid hangs in the returned [`ScrollListArea`](gdtf_ui::ScrollListArea) (the clipping,
//! scrolling viewport — NOT the scroll-list grid root frame the marker rides — the GTW-421
//! parenting rule).
//!
//! ## Lightweight dash mechanism (the C1/C2 perf clause)
//!
//! No extra nodes per cell: the per-cell dimmed dashes (C2) are each cell's own thin
//! [`Node::border`] painted a dimmed [`BorderColor`]; the dashed boundary (C1) is a SINGLE
//! thicker border on the grid container node. So a `w × h` grid is exactly `w × h` cell
//! [`ImageNode`]s plus one container — never 2+ extra nodes per cell.
//!
//! ## Rebuild model (the ui-mutate-not-respawn carve-out)
//!
//! A theme/size change DESPAWNS the [`CanvasRoot`] subtree and rebuilds it. A full rebuild
//! (rather than a per-cell mutate) is the logged choice for the canvas: a size change alters
//! the cell COUNT (entities must be added/removed), and a full teardown keeps the boundary +
//! per-cell-dash + fill invariants in one builder. The rebuild is gated on a [`Local`] tracker
//! of the `(grid_size, theme)` it was last built for, so an unrelated session mutation (a
//! palette selection) never triggers a needless rebuild.

use bevy::{
    prelude::*,
    ui::{BorderColor, FlexDirection, FlexWrap, Val, widget::ImageNode},
};
use gdtf_battle_sim::level::{
    GridHeight, GridSize, GridWidth, LevelTheme, ThemeCatalogRegistry, TileAtlasIndex,
};
use gdtf_ui::{ScrollListArea, ScrollListColors, spawn_scroll_list, theme::GdtfTheme};

use crate::{CanvasRegion, session::MapEditorSession, tile_atlas::TileAtlas};

/// One drawable cell's square edge, in screen pixels — the chosen canvas SCALE (the GTW-423
/// open design decision).
///
/// A documented framework layout const (the no-bare-types clause-4 plumbing carve-out, the
/// `tile_atlas` `TERRAIN_TILE_PX` precedent), fed to the cell [`Node`] width/height. Chosen at
/// `24` so a cell reads comfortably yet the full `60 × 60` grid (1440 px) clearly overflows the
/// centre column — which the wrapping scroll list then scrolls. NOT a per-cell SIZE the sim
/// owns (the sim is render-free); a pure presentation magnitude.
const CANVAS_CELL_PX: f32 = 24.0;

/// The per-cell grid-line border width, in screen pixels (C2) — the thin dimmed dash ruled
/// around each cell. A framework layout const (clause-4 plumbing carve-out).
const CELL_DASH_PX: f32 = 1.0;

/// The drawable-area boundary border width, in screen pixels (C1) — the thicker dashed frame
/// around the whole grid. A framework layout const (clause-4 plumbing carve-out).
const BOUNDARY_PX: f32 = 2.0;

/// Marker on the canvas's grid CONTAINER node — the single node that carries the C1 dashed
/// boundary border and holds every cell. A unit marker (no-bare-types); presence + its
/// [`CanvasExtent`] is the signal the rebuild + the test read.
///
/// It hangs inside the [`CanvasRegion`]'s [`ScrollListArea`] (the GTW-421 parenting rule). The
/// rebuild despawns it (subtree) and respawns it on a size/theme change.
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
    width:  GridWidth,
    /// The grid's y span in cells (the [`GridSize::height`] the canvas drew).
    height: GridHeight,
}

impl CanvasExtent {
    /// Build a canvas extent from a [`GridSize`]'s x/y spans (z is out of scope — 2D plane).
    #[must_use]
    const fn from_grid(size: GridSize) -> Self {
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
/// tile sprite (C3) and its own per-cell dimmed-dash border (C2). A unit marker (no-bare-types);
/// the test counts these to assert one fill per `width × height` cell.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasCell;

/// The grid + theme a [`CanvasRoot`] was last built for — the [`sync_canvas`] rebuild tracker.
///
/// A named [`Local`]-state newtype (no-bare-types): pairs the [`GridSize`] and [`LevelTheme`]
/// the canvas last drew, so the rebuild fires on EITHER a size change (new extent / cell count)
/// or a theme change (new default-floor fill) — C4 — and on neither for an unrelated session
/// mutation. `None` before the first build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CanvasBuiltFor {
    /// The grid size the canvas was last built for.
    size:  GridSize,
    /// The theme whose default floor the cells were last filled with.
    theme: LevelTheme,
}

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
/// new root under the [`CanvasRegion`]'s [`ScrollListArea`] (the GTW-421 parenting rule). The
/// last-built `(size, theme)` is tracked in a [`Local`] so an unrelated session mutation (a
/// palette selection) never triggers a needless rebuild.
pub(crate) fn sync_canvas(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    atlas: Option<Res<TileAtlas>>,
    registry: Option<Res<ThemeCatalogRegistry>>,
    session: Option<Res<MapEditorSession>>,
    roots: Query<Entity, With<CanvasRoot>>,
    mut built_for: Local<Option<CanvasBuiltFor>>,
) {
    let (Some(atlas), Some(registry), Some(session)) = (atlas, registry, session) else {
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
    spawn_canvas(&mut commands, &theme, &atlas, extent, fill_index);
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

/// Build the canvas: the [`CanvasRoot`] grid container (carrying the C1 dashed boundary + the
/// [`CanvasExtent`]) holding one [`CanvasCell`] per `width × height` cell, each pre-filled with
/// the default-floor sprite (C3) and bordered with a per-cell dimmed dash (C2). The root is
/// deferred-parented under the [`CanvasRegion`]'s [`ScrollListArea`] (the GTW-421 parenting
/// rule).
fn spawn_canvas(
    commands: &mut Commands,
    theme: &GdtfTheme,
    atlas: &TileAtlas,
    extent: CanvasExtent,
    fill_index: Option<TileAtlasIndex>,
) {
    let boundary_color = *theme.panel.border_color;
    let dash_color = dimmed(*theme.panel.border_color);
    let cells: Vec<Entity> = (0..extent.cell_count())
        .map(|_| spawn_cell(commands, atlas, fill_index, dash_color))
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

/// Spawn one drawable cell: a fixed-[`CANVAS_CELL_PX`] [`ImageNode`] pre-filled with the
/// default-floor tile sprite (C3) and ruled with a per-cell dimmed-dash border (C2).
///
/// When the default floor resolves, the cell is an [`ImageNode::from_atlas_image`] over the
/// terrain sheet at the tile's atlas index (the palette-row precedent). When it does not (a
/// theme with no catalog), the cell is a plain bordered box — boundary + dashes still render.
fn spawn_cell(
    commands: &mut Commands,
    atlas: &TileAtlas,
    fill_index: Option<TileAtlasIndex>,
    dash_color: Color,
) -> Entity {
    let mut cell = commands.spawn((CanvasCell, cell_node(), BorderColor::all(dash_color)));
    if let Some(index) = fill_index {
        cell.insert(ImageNode::from_atlas_image(
            atlas.image(),
            TextureAtlas {
                layout: atlas.layout(),
                index:  *index,
            },
        ));
    }
    cell.id()
}

/// Find the [`CanvasRegion`]'s [`ScrollListArea`] — the clipping, scrolling viewport child of
/// the region's scroll-list grid root frame (the GTW-421 parenting rule).
///
/// The [`CanvasRegion`] marker rides the [`CanvasRegion`] panel; that panel holds the scroll
/// list whose [`ScrollListArea`] is the canvas's scroll viewport. Walks the panel's descendants
/// for the area. Returns [`None`] if the area is not yet present (the deferred command runs
/// after the shell spawn applied the scroll list, so it exists by then).
fn canvas_scroll_area(world: &mut World) -> Option<Entity> {
    let region = world
        .query_filtered::<Entity, With<CanvasRegion>>()
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
        .is_ok_and(|entity| entity.contains::<ScrollListArea>())
    {
        return Some(root);
    }
    let children = world.get::<Children>(root)?;
    children
        .iter()
        .find_map(|child| find_scroll_area(world, child))
}

/// `OnEnter(Editing)`: wrap the [`CanvasRegion`] panel in a `gdtf_ui`
/// [`spawn_scroll_list`](gdtf_ui::spawn_scroll_list) so the (overflowing) cell grid SCROLLS
/// rather than squeezing (the chosen scale model).
///
/// The shell's [`CanvasRegion`] is a themed panel; this hangs a scroll list inside it (deferred,
/// so the panel exists) and re-parents the scroll-list root under the panel — the
/// `parent_scroll_root_under` precedent. [`sync_canvas`] then parents the grid under the
/// returned [`ScrollListArea`].
pub(crate) fn spawn_canvas_scroll(mut commands: Commands, theme: Res<GdtfTheme>) {
    let colors = ScrollListColors {
        area:  *theme.panel.color,
        track: *theme.panel.border_color,
        thumb: *theme.panel.border_color,
    };
    let area = spawn_scroll_list(&mut commands, colors, CanvasScroll);
    commands.queue(move |world: &mut World| {
        let Some(region) = world
            .query_filtered::<Entity, With<CanvasRegion>>()
            .iter(world)
            .next()
        else {
            return;
        };
        // The scroll-list area's root frame is parented within the same buffer by
        // `spawn_scroll_list`; move that root under the canvas region panel.
        let Some(root) = world.get::<ChildOf>(area).map(ChildOf::parent) else {
            return;
        };
        if let Ok(mut region_entity) = world.get_entity_mut(region) {
            region_entity.add_child(root);
        }
    });
}

/// Marker on the canvas's wrapping scroll-list ROOT FRAME (the `gdtf_ui` scroll list put up by
/// [`spawn_canvas_scroll`]). A unit marker (no-bare-types) so a test can find the scroll list.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasScroll;

/// The grid CONTAINER [`Node`] (C1/C2): a flex-WRAP row sized to exactly `width` cells across so
/// the cells wrap into `height` rows, carrying the thick C1 boundary border. Fixed-px sizing
/// (the chosen scale model) — the only place the editor uses `Px`, because the canvas is a
/// pixel-true cell grid that scrolls, not a responsive panel.
fn grid_container_node(extent: CanvasExtent) -> Node {
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
fn cell_node() -> Node {
    Node {
        width: Val::Px(CANVAS_CELL_PX),
        height: Val::Px(CANVAS_CELL_PX),
        border: UiRect::all(Val::Px(CELL_DASH_PX)),
        ..default()
    }
}

/// Dim a theme color toward transparency for the per-cell grid-line dash (C2) — the per-cell
/// rules read as faint guides, distinct from the thicker opaque C1 boundary. Halves the alpha.
fn dimmed(color: Color) -> Color {
    let rgba = color.to_srgba();
    Color::srgba(rgba.red, rgba.green, rgba.blue, rgba.alpha * 0.5)
}

#[cfg(test)]
mod tests {
    use bevy::ui::Val;
    use gdtf_battle_sim::level::{GridHeight, GridWidth};

    use super::{BOUNDARY_PX, CANVAS_CELL_PX, CELL_DASH_PX, CanvasExtent, grid_container_node};

    /// T6 (GTW-463 C2): layout-guard — [`grid_container_node`]'s row-width must pack EXACTLY
    /// `width` cells per row for sizes that expose the old `BorderBox` box-model bug.
    ///
    /// **What this tests:** `grid_container_node` sets the container's `Node.width` to
    /// `Val::Px(2*BOUNDARY_PX + cell_outer * width)`. The container's inner content width is
    /// therefore `cell_outer * width`. Under Bevy's default `BorderBox` each cell's flex
    /// footprint equals `CANVAS_CELL_PX` (24 px) — the 1 px dash border paints INSIDE the
    /// 24 px — so the footprint is NOT `CANVAS_CELL_PX + 2*CELL_DASH_PX = 26 px`.
    ///
    /// **Pre-fix failure (the bug):** the old formula used `cell_outer = 26 px`, so the
    /// container outer width was `4 + 26*W` px and inner was `26*W`. Taffy flex-wraps at
    /// `floor(26*W / 24)` cells per row: at W = 16 that is `17 ≠ 16` (right-overflow +
    /// ragged bottom-right). At W = 8 it was `8` — a lucky fit that masked the bug in QA.
    ///
    /// **Post-fix correctness:** `cell_outer = CANVAS_CELL_PX = 24 px`, so the container
    /// outer width is `4 + 24*W` px and inner is `24*W`. `floor(24*W / 24) = W` exactly.
    ///
    /// **Why this test catches a revert:** it calls the REAL `grid_container_node` and
    /// extracts the `Val::Px` width it sets on the container. Reverting the fix (restoring
    /// `cell_outer = 26 px`) changes the `Val::Px` value and breaks the `assert_eq!`.
    /// The `assert_ne!` regression guard also confirms the BUG formula differs from `width`
    /// at these widths — so if someone "fixes" the bug differently (e.g. with a different
    /// constant), both sides of the check remain meaningful.
    #[test]
    fn grid_row_packs_exactly_width_cells_per_row() {
        // The cell's real flex footprint under BorderBox:
        // dash border (CELL_DASH_PX) is INSIDE the 24 px cell — footprint = CANVAS_CELL_PX.
        let cell_footprint = CANVAS_CELL_PX;

        // The OLD (buggy) `cell_outer`: CANVAS_CELL_PX + 2*CELL_DASH_PX = 26 px.
        let buggy_cell_outer = 2.0f32.mul_add(CELL_DASH_PX, CANVAS_CELL_PX);

        // For each bug-exposing width, call the REAL grid_container_node, extract its px width,
        // derive the inner content width, and assert it packs exactly `width` cells per row.
        for width in [16_u8, 20, 30, 60] {
            let extent = CanvasExtent {
                width:  GridWidth::new(width),
                height: GridHeight::new(8), // height does not affect the row-width formula
            };
            let node = grid_container_node(extent);

            // Extract the pixel width the node declares; it must be Val::Px for a
            // fixed-scale pixel grid. Use a sentinel so the assertion below can still
            // fire a meaningful message if the variant changes unexpectedly.
            let outer_px = if let Val::Px(px) = node.width {
                px
            } else {
                // Signal a non-Px width via a sentinel: cells_per_row below will be 0 and
                // the assert_eq will fail with a message identifying width + the bad variant.
                f32::NAN
            };
            assert!(
                outer_px.is_finite(),
                "grid_container_node must set Val::Px width, got {:?} at width {width} \
                 (GTW-463 C2 guard)",
                node.width,
            );

            // Inner content width = outer - 2*BOUNDARY_PX (the container's own border).
            let inner_px = 2.0f32.mul_add(-BOUNDARY_PX, outer_px);

            // Cells per row = floor(inner_content_width / cell_footprint).
            // This is the formula taffy uses for flex-wrap row breaks.
            let cells_per_row = (inner_px / cell_footprint).floor() as u8;
            assert_eq!(
                cells_per_row, width,
                "grid_container_node at width {width}: inner width {inner_px}px must pack \
                 exactly {width} cells per row at footprint {cell_footprint}px (GTW-463 C2)",
            );

            // Regression guard: the BUG formula (26*W) must pack ≠ width cells at these
            // widths — confirming the bug existed and the fix actually changes the outcome.
            // (Width 8 is excluded from the loop: floor(26*8/24)=8 was a lucky fit.)
            let buggy_inner = buggy_cell_outer * f32::from(width);
            let buggy_cells_per_row = (buggy_inner / cell_footprint).floor() as u8;
            assert_ne!(
                buggy_cells_per_row, width,
                "pre-fix regression guard: the old formula's inner width {buggy_inner}px should \
                 pack {buggy_cells_per_row} cells per row at width {width}, NOT {width} — if \
                 this fires, adjust the test width list (width is a lucky fit for the old formula)",
            );
        }
    }
}
