use bevy_egui::egui;
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::facing::TerrainFacing,
};

use super::support::{COVER, SLAB, registry, size};
use crate::{
    editor_map::EditorMap,
    egui_shell::prefab::level_rail::{cache::RailThumbCache, occupancy::storey_key},
};

#[test]
fn signature_keys_content_not_ticks() {
    let size = size();
    let reg = registry();
    let mut map = EditorMap::new();
    let slot = CellLevel::new(Cell::new(1, 1), Level::new(0));

    let (empty_a, _) = storey_key(&map, Some(&reg), size, Level::new(0));
    let (empty_b, _) = storey_key(&map, Some(&reg), size, Level::new(0));
    assert_eq!(empty_a, empty_b, "identical content keys identically");

    assert!(map.paint_at(slot, COVER, TerrainFacing::default(), size));
    let (painted, _) = storey_key(&map, Some(&reg), size, Level::new(0));
    assert_ne!(empty_a, painted, "a paint moves the key");

    assert!(map.paint_at(slot, SLAB, TerrainFacing::default(), size));
    let (repainted, _) = storey_key(&map, Some(&reg), size, Level::new(0));
    assert_ne!(
        painted, repainted,
        "repainting the SAME cell with another role moves the key (hue is content)"
    );

    let narrower = GridSize::new(GridWidth::new(3), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default());
    let (resized, _) = storey_key(&map, Some(&reg), narrower, Level::new(0));
    assert_ne!(
        repainted, resized,
        "a grid resize moves the key even with an unchanged painted set"
    );
}

#[test]
fn cache_rebuilds_only_on_signature_change() {
    let size = size();
    let reg = registry();
    let ctx = egui::Context::default();
    let mut cache = RailThumbCache::default();
    let mut map = EditorMap::new();
    let builds = std::cell::Cell::new(0_u32);
    let build = || {
        builds.set(builds.get() + 1);
        egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE])
    };

    let (sig_empty, count_empty) = storey_key(&map, Some(&reg), size, Level::new(0));
    let thumb = cache.refresh(&ctx, Level::new(0), sig_empty, count_empty, build);
    assert!(thumb.is_some(), "a refreshed storey always yields a thumb");
    assert_eq!(builds.get(), 1, "the first probe builds");

    let build = || {
        builds.set(builds.get() + 1);
        egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE])
    };
    let hit = cache.refresh(&ctx, Level::new(0), sig_empty, count_empty, build);
    assert_eq!(
        builds.get(),
        1,
        "an unchanged signature reuses (C2 — no rebuild)"
    );
    assert_eq!(
        hit.map(|thumb| thumb.count),
        Some(count_empty),
        "a hit serves the cached count"
    );

    assert!(map.paint_at(
        CellLevel::new(Cell::new(0, 0), Level::new(0)),
        COVER,
        TerrainFacing::default(),
        size
    ));
    let (sig_painted, count_painted) = storey_key(&map, Some(&reg), size, Level::new(0));
    let build = || {
        builds.set(builds.get() + 1);
        egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE])
    };
    let rebuilt = cache.refresh(&ctx, Level::new(0), sig_painted, count_painted, build);
    assert_eq!(builds.get(), 2, "a moved signature rebuilds");
    assert_eq!(
        rebuilt.map(|thumb| thumb.count),
        Some(count_painted),
        "the rebuild stores the fresh count"
    );

    for storey in [1_u8, 2] {
        let build = || {
            builds.set(builds.get() + 1);
            egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE])
        };
        let (sig, count) = storey_key(&map, Some(&reg), size, Level::new(storey));
        cache.refresh(&ctx, Level::new(storey), sig, count, build);
    }
    assert_eq!(cache.len(), 3, "three storeys cached before the shrink");
    let one_level = GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(1))
        .unwrap_or_else(|_| GridSize::default());
    cache.prune(one_level);
    assert_eq!(cache.len(), 1, "pruning drops the vanished storeys' thumbs");
}
