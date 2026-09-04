use std::hash::{DefaultHasher, Hash, Hasher};

use bevy_egui::egui;
use gdtf_battle_sim::{
    level::GridSize,
    metric::{CellLevel, Level},
    terrain::{
        def::{TerrainDef, TerrainDefRegistry, TerrainTag, TerrainUuid},
        entity::TerrainPieceKind,
    },
};

use crate::{
    editor_map::{EditorMap, PaintedPiece},
    placement::names_a_ladder,
};

pub(super) const FALLBACK_HUE: egui::Color32 = egui::Color32::from_rgb(200, 60, 200);

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
        .map_or(FALLBACK_HUE, rail_hue)
}

// The kind and the two tags the rail tells apart, plus the ladder its display name names.
fn rail_hue(def: &TerrainDef) -> egui::Color32 {
    if names_a_ladder(&def.display_name) {
        return egui::Color32::from_rgb(210, 200, 80);
    }
    if def.tags.contains(&TerrainTag::Openable) {
        return egui::Color32::from_rgb(60, 160, 150);
    }
    if def.tags.contains(&TerrainTag::Stair) {
        return egui::Color32::from_rgb(90, 200, 90);
    }
    match def.sim_kind.kind() {
        TerrainPieceKind::Wall => egui::Color32::from_rgb(200, 200, 205),
        TerrainPieceKind::Cover => egui::Color32::from_rgb(190, 150, 60),
        TerrainPieceKind::Emplacement => egui::Color32::from_rgb(200, 90, 40),
        TerrainPieceKind::Slab => egui::Color32::from_rgb(110, 130, 160),
    }
}
