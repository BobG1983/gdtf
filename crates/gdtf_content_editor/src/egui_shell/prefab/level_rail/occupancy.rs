//! The rail's **pure CPU occupancy sweep** (GTW-595 C1/C2) — one storey of the sparse
//! [`EditorMap`] rendered to solid texels, plus the change-key its thumbnail cache
//! rebuilds on.
//!
//! ## Role-hued through the presenter's ONE resolution (no parallel palette)
//!
//! A painted cell's texel hue resolves EXACTLY the way the preview resolves a sprite
//! (GTW-495 / [`terrain_atlas_index`](crate::terrain_graphics::terrain_atlas_index)):
//! [`TerrainUuid`] → registry def → [`graphic_key`] → [`TileRole::from_key`] (the ONE
//! key↔role site — GTW-566). The rail stops at the classified [`TileRole`] instead of
//! reading the atlas index, because a thumbnail texel is a flat colour, not a 16×16
//! sprite; the hue is then an exhaustive [`rail_hue`] match over that CLOSED vocabulary,
//! so there is no second key table to drift and an out-of-vocabulary def falls back to
//! the loud [`FALLBACK_HUE`] rather than vanishing.
//!
//! ## The change-key
//!
//! [`storey_key`] folds every in-bounds painted cell of the storey — position AND
//! resolved hue — plus the grid's x/y extent into one order-independent
//! [`StoreySignature`]. A paint, clear, repaint-with-another-role, registry hot-reload
//! that re-keys a def's graphic, or a grid resize therefore all change the signature,
//! and NOTHING else does — the cache (GTW-595 C2) rebuilds a storey's texture exactly
//! when its rendered content changed.

use std::hash::{DefaultHasher, Hash, Hasher};

use bevy_egui::egui;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    level::GridSize,
    metric::{CellLevel, Level},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::{editor_map::EditorMap, terrain_graphics::graphic_key};

/// The loud fallback texel hue for a painted cell whose [`TerrainUuid`] resolves to no
/// registered def or an out-of-vocabulary graphic role — magenta, unmistakably "unresolved"
/// (the sprite path's equivalent falls back to the theme default draw; a thumbnail texel
/// has no sprite to fall back to, so it flags instead of hiding the paint).
const FALLBACK_HUE: egui::Color32 = egui::Color32::from_rgb(200, 60, 200);

/// One storey's occupancy CHANGE-KEY (GTW-595 C2) — an order-independent fold of the
/// storey's in-bounds painted cells (position + resolved hue) and the grid's x/y extent.
///
/// A named newtype over the folded hash (no-bare-types: the rebuild key is a domain
/// value, not a loose `u64`); private inner, compared whole via `PartialEq`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct StoreySignature(u64);

/// How many cells the author has painted on one storey (in-bounds under the CURRENT
/// grid) — the per-row count the rail displays (GTW-595 C1).
///
/// A named newtype over the tally (no-bare-types), [`Deref`](std::ops::Deref)ing to it
/// for display.
#[derive(bevy::prelude::Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PaintedCount(usize);

/// Compute one storey's [`StoreySignature`] + [`PaintedCount`] in a single O(painted)
/// pass over the sparse model — the per-frame probe the thumbnail cache compares before
/// deciding to rebuild (GTW-595 C2). Only in-bounds cells count: the model may retain
/// paints from before a grid shrink, and those render nowhere.
pub(crate) fn storey_key(
    map: &EditorMap,
    registry: Option<&TerrainDefRegistry>,
    size: GridSize,
    storey: Level,
) -> (StoreySignature, PaintedCount) {
    let mut folded: u64 = 0;
    let mut count: usize = 0;
    for (slot, tile) in painted_in_storey(map, storey) {
        if texel_index(slot, size).is_none() {
            continue;
        }
        let mut hasher = DefaultHasher::new();
        slot.x.hash(&mut hasher);
        slot.y.hash(&mut hasher);
        cell_hue(registry, tile).to_array().hash(&mut hasher);
        // wrapping_add is commutative, so the fold is independent of the HashMap's
        // unspecified iteration order.
        folded = folded.wrapping_add(hasher.finish());
        count += 1;
    }
    // Mix the x/y extent in so a grid RESIZE re-keys every storey (the texel grid's
    // dimensions changed even where the painted set did not).
    let mut hasher = DefaultHasher::new();
    (*size.width(), *size.height()).hash(&mut hasher);
    hasher.write_u64(folded);
    (StoreySignature(hasher.finish()), PaintedCount(count))
}

/// Sweep one storey of the sparse [`EditorMap`] into a `width × height`
/// [`egui::ColorImage`] — one texel per drawable cell, row-major with cell `(0, 0)` at
/// the image's TOP-LEFT (grid `+y` renders down-screen, matching the preview viewport's
/// orientation). Painted in-bounds cells draw their role hue as a SOLID texel; unpainted
/// cells stay transparent (the sweep reads ONLY the sparse painted set — the ground
/// storey's theme default-floor fill is a preview concern, not authored occupancy).
pub(crate) fn storey_image(
    map: &EditorMap,
    registry: Option<&TerrainDefRegistry>,
    size: GridSize,
    storey: Level,
) -> egui::ColorImage {
    let width = usize::from(*size.width());
    let height = usize::from(*size.height());
    let mut pixels = vec![egui::Color32::TRANSPARENT; width * height];
    for (slot, tile) in painted_in_storey(map, storey) {
        if let Some(index) = texel_index(slot, size)
            && let Some(pixel) = pixels.get_mut(index)
        {
            *pixel = cell_hue(registry, tile);
        }
    }
    egui::ColorImage::new([width, height], pixels)
}

/// Every painted `(slot, tile)` on `storey` — the sparse model filtered to one z plane
/// (O(painted), never O(volume): the sweep iterates the map's entries, not the grid).
fn painted_in_storey(
    map: &EditorMap,
    storey: Level,
) -> impl Iterator<Item = (&CellLevel, &TerrainUuid)> {
    let z = i32::from(*storey);
    map.painted().filter(move |(slot, _)| slot.z == z)
}

/// The row-major texel index of `slot` in a `width × height` thumbnail, or [`None`] when
/// the cell falls outside the CURRENT grid's x/y extent (stale paints from before a
/// shrink render nowhere — the same in-bounds rule the drawable volume enforces).
fn texel_index(slot: &CellLevel, size: GridSize) -> Option<usize> {
    let x = usize::try_from(slot.x).ok()?;
    let y = usize::try_from(slot.y).ok()?;
    let width = usize::from(*size.width());
    let height = usize::from(*size.height());
    (x < width && y < height).then_some(y * width + x)
}

/// Resolve a painted [`TerrainUuid`] to its texel hue THE WAY THE PREVIEW RESOLVES its
/// sprite (GTW-595 C1): registry def → [`graphic_key`] → [`TileRole::from_key`] — the
/// same chain [`terrain_atlas_index`](crate::terrain_graphics::terrain_atlas_index)
/// walks, stopping at the classified role (a texel needs a colour, not an atlas index).
/// An absent registry, an unregistered key, or an out-of-vocabulary role all fall back
/// to the loud [`FALLBACK_HUE`] — the cell still reads as painted.
fn cell_hue(registry: Option<&TerrainDefRegistry>, tile: &TerrainUuid) -> egui::Color32 {
    registry
        .and_then(|registry| registry.def(tile))
        .and_then(|def| TileRole::from_key(graphic_key(def)))
        .map_or(FALLBACK_HUE, rail_hue)
}

/// The thumbnail hue of one graphic role — the display leaf over the presenter's CLOSED
/// [`TileRole`] vocabulary (GTW-566), exhaustive so a new role forces an explicit hue
/// choice here at compile time. Hue families group by what the role IS (floors muted,
/// walls light, cover ochre, connectors green, doors teal), so storeys read apart at a
/// glance in the rail.
const fn rail_hue(role: TileRole) -> egui::Color32 {
    match role {
        TileRole::Floor => egui::Color32::from_rgb(100, 110, 90),
        TileRole::FloorAltPanel => egui::Color32::from_rgb(105, 105, 115),
        TileRole::Wall => egui::Color32::from_rgb(200, 200, 205),
        TileRole::WallEw => egui::Color32::from_rgb(170, 170, 180),
        TileRole::Cover => egui::Color32::from_rgb(190, 150, 60),
        TileRole::Emplacement => egui::Color32::from_rgb(200, 90, 40),
        TileRole::EmplacementOccupied => egui::Color32::from_rgb(170, 60, 40),
        TileRole::Slab => egui::Color32::from_rgb(110, 130, 160),
        TileRole::Rubble => egui::Color32::from_rgb(130, 100, 70),
        TileRole::SlabDestroyed => egui::Color32::from_rgb(100, 80, 60),
        TileRole::Door | TileRole::DoorNs => egui::Color32::from_rgb(60, 160, 150),
        TileRole::DoorEw => egui::Color32::from_rgb(50, 140, 135),
        TileRole::StairUp | TileRole::StairNsUp | TileRole::StairEwUp => {
            egui::Color32::from_rgb(90, 200, 90)
        }
        TileRole::StairDown | TileRole::StairNsDown | TileRole::StairEwDown => {
            egui::Color32::from_rgb(60, 140, 60)
        }
        TileRole::Ladder => egui::Color32::from_rgb(210, 200, 80),
    }
}
