use std::hash::{DefaultHasher, Hash, Hasher};

use bevy_egui::egui;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    level::GridSize,
    metric::{CellLevel, Level},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::{
    editor_map::{EditorMap, PaintedPiece},
    terrain_graphics::graphic_key,
};

const FALLBACK_HUE: egui::Color32 = egui::Color32::from_rgb(200, 60, 200);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct StoreySignature(u64);

#[derive(bevy::prelude::Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PaintedCount(usize);

pub(crate) fn storey_key(
    map: &EditorMap,
    registry: Option<&TerrainDefRegistry>,
    size: GridSize,
    storey: Level,
) -> (StoreySignature, PaintedCount) {
    let mut folded: u64 = 0;
    let mut count: usize = 0;
    for (slot, piece) in painted_in_storey(map, storey) {
        if texel_index(slot, size).is_none() {
            continue;
        }
        let mut hasher = DefaultHasher::new();
        slot.x.hash(&mut hasher);
        slot.y.hash(&mut hasher);
        cell_hue(registry, &piece.tile())
            .to_array()
            .hash(&mut hasher);
        folded = folded.wrapping_add(hasher.finish());
        count += 1;
    }
    let mut hasher = DefaultHasher::new();
    (*size.width(), *size.height()).hash(&mut hasher);
    hasher.write_u64(folded);
    (StoreySignature(hasher.finish()), PaintedCount(count))
}

pub(crate) fn storey_image(
    map: &EditorMap,
    registry: Option<&TerrainDefRegistry>,
    size: GridSize,
    storey: Level,
) -> egui::ColorImage {
    let width = usize::from(*size.width());
    let height = usize::from(*size.height());
    let mut pixels = vec![egui::Color32::TRANSPARENT; width * height];
    for (slot, piece) in painted_in_storey(map, storey) {
        if let Some(index) = texel_index(slot, size)
            && let Some(pixel) = pixels.get_mut(index)
        {
            *pixel = cell_hue(registry, &piece.tile());
        }
    }
    egui::ColorImage::new([width, height], pixels)
}

fn painted_in_storey(
    map: &EditorMap,
    storey: Level,
) -> impl Iterator<Item = (&CellLevel, &PaintedPiece)> {
    let z = i32::from(*storey);
    map.painted().filter(move |(slot, _)| slot.z == z)
}

fn texel_index(slot: &CellLevel, size: GridSize) -> Option<usize> {
    let x = usize::try_from(slot.x).ok()?;
    let y = usize::try_from(slot.y).ok()?;
    let width = usize::from(*size.width());
    let height = usize::from(*size.height());
    (x < width && y < height).then_some(y * width + x)
}

fn cell_hue(registry: Option<&TerrainDefRegistry>, tile: &TerrainUuid) -> egui::Color32 {
    registry
        .and_then(|registry| registry.def(tile))
        .and_then(|def| TileRole::from_key(graphic_key(def)))
        .map_or(FALLBACK_HUE, rail_hue)
}

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
