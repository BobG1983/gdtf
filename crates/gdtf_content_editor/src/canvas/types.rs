//! Component types, layout constants, node builders, and tile-resolution helpers for the
//! map-editor canvas (swept onto the UUID-keyed terrain model in GTW-495).
//!
//! The unit-marker types ([`CanvasRoot`], [`CanvasCell`], [`CanvasGhost`], [`CanvasScroll`]),
//! the rebuild-tracker ([`CanvasBuiltFor`]), the extent value-type ([`CanvasExtent`]), the three
//! layout constants ([`CANVAS_CELL_PX`], [`CELL_DASH_PX`], [`BOUNDARY_PX`]), the node-builder
//! functions, the cell-spawner ([`spawn_cell`]), and the shared tile-index resolver
//! ([`paint_index_for`]) all live here.

use bevy::{
    prelude::*,
    ui::{BorderColor, FlexDirection, FlexWrap, UiTransform, Val, widget::ImageNode},
};
use gdtf_battle_presenter::{TileIndex, TileRoles};
use gdtf_battle_sim::{
    Cell,
    level::{GridHeight, GridSize, GridWidth, ThemeUuid},
    metric::{CellLevel, Level},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::{
    editor_map::EditorMap, session::MapEditorSession, terrain_graphics::terrain_atlas_index,
    tile_atlas::TileAtlas,
};

/// One drawable cell's square edge, in screen pixels — the chosen canvas SCALE (the GTW-423
/// open design decision).
///
/// A documented framework layout const (the no-bare-types clause-4 plumbing carve-out, the
/// `tile_atlas` `TERRAIN_TILE_PX` precedent), fed to the cell [`Node`] width/height. Chosen at
/// `24` so a cell reads comfortably yet the full `60 × 60` grid (1440 px) clearly overflows the
/// centre column — which the wrapping scroll list then scrolls.
pub(super) const CANVAS_CELL_PX: f32 = 24.0;

/// The per-cell grid-line border width, in screen pixels (C2). A framework layout const.
pub(super) const CELL_DASH_PX: f32 = 1.0;

/// The drawable-area boundary border width, in screen pixels (C1). A framework layout const.
pub(super) const BOUNDARY_PX: f32 = 2.0;

/// Marker on the canvas's grid CONTAINER node — the single node that carries the C1 dashed
/// boundary border and holds every cell. A unit marker (no-bare-types); presence + its
/// [`CanvasExtent`] is the signal the rebuild + the test read.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasRoot;

/// The grid extent the [`CanvasRoot`] was built for — its `width × height` cell counts (C1),
/// carried on the root so the boundary's extent is readable without re-deriving it.
///
/// A named newtype-bearing component (no-bare-types): the two spans are the sim's own
/// [`GridWidth`] / [`GridHeight`] newtypes. Z (levels) is excluded — 2D x/y only.
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
    #[must_use]
    pub fn cell_count(&self) -> usize {
        usize::from(*self.width) * usize::from(*self.height)
    }
}

/// Marker on one drawable CELL — a pre-filled [`ImageNode`] carrying the theme's default-floor
/// tile sprite (C3) and its own per-cell dimmed-dash border (C2).
///
/// Carries the cell's ground-plane [`Cell`] coordinate (GTW-426): the canvas spawns cells in
/// row-major order, so cell index `i` is `(x = i % width, y = i / width)`. A `Button` is
/// `#[require]`d so the engine's `ui_focus_system` drives its [`Interaction`] (bevy-traps #6).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasCell {
    /// The cell's ground-plane coordinate (`x` in `0..width`, `y` in `0..height`).
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
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasGhost;

/// The grid + theme a [`CanvasRoot`] was last built for — the [`sync_canvas`](super::sync::sync_canvas)
/// rebuild tracker.
///
/// A named [`Local`]-state newtype (no-bare-types): pairs the [`GridSize`], the [`ThemeUuid`], and
/// the [`Level`] storey the canvas last drew, so the rebuild fires on a size change, a theme change
/// (C4), OR a level step (GTW-500 C1) — and on none for an unrelated session mutation. `None`
/// before the first build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CanvasBuiltFor {
    /// The grid size the canvas was last built for.
    pub(crate) size:  GridSize,
    /// The theme whose default floor the cells were last filled with.
    pub(crate) theme: ThemeUuid,
    /// The storey the cells were last drawn for (GTW-500 C1).
    pub(crate) level: Level,
}

/// Marker on the canvas's wrapping scroll-list ROOT FRAME. A unit marker (no-bare-types) so a
/// test can find the scroll list.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanvasScroll;

/// Marker on the CANVAS's [`ScrollListArea`](gdtf_ui::ScrollListArea) cell — the clipping,
/// scrolling viewport the cell grid hangs inside (GTW-500).
///
/// The editor spawns THREE [`ScrollListArea`](gdtf_ui::ScrollListArea)s (the left palette, the
/// canvas, the right panel) and that marker is identical across all three; the per-region
/// discriminator ([`CanvasScroll`] / `LeftPaletteRegion` / `RightPanelRegion`) rides the scroll-list
/// ROOT FRAME, NOT the area cell. So the centring / zoom-gate / zoom-relayout systems — which must
/// act on the canvas's viewport and NO other — discriminate it by THIS marker, inserted on the
/// returned area entity in [`spawn_canvas_scroll`](super::scroll::spawn_canvas_scroll). A unit
/// marker (no-bare-types). Crate-internal — only the editor's own canvas systems discriminate on it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CanvasScrollArea;

/// The grid CONTAINER [`Node`] (C1/C2): a flex-WRAP row sized to exactly `width` cells across so
/// the cells wrap into `height` rows, carrying the thick C1 boundary border. Fixed-px sizing
/// (the chosen scale model).
pub(super) fn grid_container_node(extent: CanvasExtent) -> Node {
    // Bevy's default `BoxSizing` is `BorderBox`, so the per-cell dash border paints INSIDE the
    // 24 px cell footprint — each cell's real outer edge is still `CANVAS_CELL_PX`.
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

/// Dim a theme color toward transparency for the per-cell grid-line dash (C2). Halves the alpha.
pub(super) fn dimmed(color: Color) -> Color {
    let rgba = color.to_srgba();
    Color::srgba(rgba.red, rgba.green, rgba.blue, rgba.alpha * 0.5)
}

/// Resolve a PAINTED terrain's atlas index — the index a painted cell's fill sprite shows (C2),
/// resolved THE WAY THE PRESENTER DOES (the def's `presenter_kind.graphic_name` against
/// [`TileRoles`]).
///
/// Looks the painted [`TerrainUuid`] up in the [`TerrainDefRegistry`] and resolves its graphic
/// role through the presenter's [`TileRoles`] table, or [`None`] if the key names no def / its
/// graphic role is out of vocabulary (the cell then falls back to the default-floor fill rather
/// than panicking). Shared by sync, paint, and ghost. The `_theme` is retained for the shared
/// signature even though a UUID resolves a terrain without it.
pub(super) fn paint_index_for(
    registry: &TerrainDefRegistry,
    roles: &TileRoles,
    _theme: ThemeUuid,
    key: TerrainUuid,
) -> Option<TileIndex> {
    terrain_atlas_index(registry, roles, &key)
}

/// Spawn one drawable cell at ground-plane coordinate `cell`: a fixed-[`CANVAS_CELL_PX`]
/// clickable [`Button`] [`ImageNode`] pre-filled with the default-floor tile sprite (C3) and
/// ruled with a per-cell dimmed-dash border (C2).
pub(super) fn spawn_cell(
    commands: &mut Commands,
    atlas: &TileAtlas,
    fill_index: Option<TileIndex>,
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
/// PAINTED terrain from the [`EditorMap`] model where painted (C2) and the theme default-floor
/// sprite otherwise (C3). The root is deferred-parented under the
/// [`CanvasRegion`](crate::CanvasRegion)'s [`ScrollListArea`] (the GTW-421 parenting rule).
#[expect(
    clippy::too_many_arguments,
    reason = "the canvas build threads theme + atlas + the terrain registry + the role table + \
              the session + the paint model + extent + the resolved default-floor index; each is \
              a distinct concern and collapsing them into a struct would not reduce coupling"
)]
pub(super) fn spawn_canvas(
    commands: &mut Commands,
    theme: &gdtf_ui::theme::GdtfTheme,
    atlas: &TileAtlas,
    registry: &TerrainDefRegistry,
    roles: &TileRoles,
    session: &MapEditorSession,
    map: &EditorMap,
    extent: CanvasExtent,
    level: Level,
    fill_index: Option<TileIndex>,
) {
    let boundary_color = *theme.panel.border_color;
    let dash_color = dimmed(*theme.panel.border_color);
    let width = i32::from(*extent.width());
    let cells: Vec<Entity> = (0..extent.cell_count())
        .map(|index| {
            // Row-major: cell `index` is the cell `(x = index % width, y = index / width)` on the
            // current storey. The grid container is a `flex_wrap` row of exactly `width` cells.
            #[expect(
                clippy::cast_possible_wrap,
                clippy::cast_possible_truncation,
                reason = "index < cell_count = width*height <= 60*60 = 3600, well within i32"
            )]
            let i = index as i32;
            let cell = Cell::new(i % width, i / width);
            // A painted cell ON THE CURRENT STOREY (in the model) shows its painted terrain's atlas
            // index (C2 persistence across rebuild; GTW-500 C1 level-aware); an unpainted slot falls
            // back to the default-floor (C3).
            let cell_index = map
                .tile_at_level(CellLevel::new(cell, level))
                .and_then(|key| paint_index_for(registry, roles, session.theme(), key))
                .or(fill_index);
            spawn_cell(commands, atlas, cell_index, dash_color, cell)
        })
        .collect();
    let root = commands
        .spawn((
            CanvasRoot,
            extent,
            // GTW-500 C2: the centring lever. The grid starts at IDENTITY (flex-start, reachable by
            // scroll on overflow — the GTW-421 trap); `center_canvas` nudges the translation to
            // centre it WHEN it is smaller than the viewport (never `justify`/`align: Center`).
            UiTransform::IDENTITY,
            // GTW-500 C3: the x cell-count the zoom re-layout multiplies by the new cell edge to
            // recompute the row width, without re-reading `CanvasExtent` (a disjoint root query).
            super::zoom::CanvasExtentSpan::new(u16::from(*extent.width())),
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
fn canvas_scroll_area(world: &mut World) -> Option<Entity> {
    let region = world
        .query_filtered::<Entity, With<crate::CanvasRegion>>()
        .iter(world)
        .next()?;
    find_scroll_area(world, region)
}

/// Walk a subtree (depth-first from `root`) for the first [`ScrollListArea`] descendant.
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
