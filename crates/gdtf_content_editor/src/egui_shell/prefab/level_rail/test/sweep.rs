//! A1 — the pure CPU occupancy sweep: storey scoping, role-hued texels, and the
//! current-extent bounds rule.

use bevy_egui::egui;
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::{CellLevel, Level},
    prelude::Cell,
};

use super::support::{COVER, SLAB, UNKNOWN, registry, size, texel};
use crate::{
    editor_map::EditorMap,
    egui_shell::prefab::level_rail::occupancy::{storey_image, storey_key},
};

/// The sweep is storey-scoped: each storey counts + draws ONLY its own painted cells,
/// at the row-major top-left-origin texel of its `(x, y)`.
#[test]
fn sweep_is_storey_scoped_and_counts() {
    let size = size();
    let mut map = EditorMap::new();
    assert!(map.paint_at(CellLevel::new(Cell::new(1, 2), Level::new(0)), COVER, size));
    assert!(map.paint_at(CellLevel::new(Cell::new(0, 0), Level::new(1)), SLAB, size));
    assert!(map.paint_at(CellLevel::new(Cell::new(3, 3), Level::new(1)), COVER, size));
    let reg = registry();

    let (_, ground_count) = storey_key(&map, Some(&reg), size, Level::new(0));
    let (_, upper_count) = storey_key(&map, Some(&reg), size, Level::new(1));
    assert_eq!(*ground_count, 1, "L0 holds exactly its own painted cell");
    assert_eq!(
        *upper_count, 2,
        "L1 holds exactly its own two painted cells"
    );

    let ground = storey_image(&map, Some(&reg), size, Level::new(0));
    assert_eq!(ground.size, [4, 4], "one texel per drawable cell");
    assert_ne!(
        texel(&ground, 1, 2),
        egui::Color32::TRANSPARENT,
        "the painted L0 cell draws a solid texel at its (x, y)"
    );
    assert_eq!(
        texel(&ground, 0, 0),
        egui::Color32::TRANSPARENT,
        "a cell painted ONLY on L1 stays transparent on L0 (storey-scoped)"
    );

    let upper = storey_image(&map, Some(&reg), size, Level::new(1));
    assert_ne!(texel(&upper, 0, 0), egui::Color32::TRANSPARENT);
    assert_ne!(texel(&upper, 3, 3), egui::Color32::TRANSPARENT);
    assert_eq!(
        texel(&upper, 1, 2),
        egui::Color32::TRANSPARENT,
        "the L0 paint does not leak up to L1"
    );
}

/// Texels are ROLE-hued through the preview's resolution chain: two defs with different
/// graphic roles draw different hues, the same role draws the same hue, and an
/// unresolvable key still draws (the loud fallback — a paint is never hidden).
#[test]
fn sweep_hues_by_resolved_role() {
    let size = size();
    let mut map = EditorMap::new();
    assert!(map.paint_at(CellLevel::new(Cell::new(0, 0), Level::new(0)), COVER, size));
    assert!(map.paint_at(CellLevel::new(Cell::new(1, 0), Level::new(0)), SLAB, size));
    assert!(map.paint_at(CellLevel::new(Cell::new(2, 0), Level::new(0)), COVER, size));
    assert!(map.paint_at(
        CellLevel::new(Cell::new(3, 0), Level::new(0)),
        UNKNOWN,
        size
    ));
    let reg = registry();

    let image = storey_image(&map, Some(&reg), size, Level::new(0));
    let cover_hue = texel(&image, 0, 0);
    let slab_hue = texel(&image, 1, 0);
    assert_ne!(
        cover_hue, slab_hue,
        "distinct graphic roles resolve to distinct hues"
    );
    assert_eq!(
        texel(&image, 2, 0),
        cover_hue,
        "the same role always resolves to the same hue (one resolution, no drift)"
    );
    assert_ne!(
        texel(&image, 3, 0),
        egui::Color32::TRANSPARENT,
        "an unregistered key still reads as painted (fallback hue, never hidden)"
    );
}

/// Paints stranded outside the CURRENT grid (a shrink after painting) render nowhere:
/// excluded from the texels AND the count.
#[test]
fn sweep_ignores_out_of_bounds_stale_paints() {
    let big = size();
    let mut map = EditorMap::new();
    assert!(map.paint_at(CellLevel::new(Cell::new(3, 3), Level::new(0)), COVER, big));
    let small = GridSize::new(GridWidth::new(2), GridHeight::new(2), GridLevels::new(1))
        .unwrap_or_else(|_| GridSize::default());
    let reg = registry();

    let (_, count) = storey_key(&map, Some(&reg), small, Level::new(0));
    assert_eq!(*count, 0, "a stale out-of-extent paint counts for nothing");
    let image = storey_image(&map, Some(&reg), small, Level::new(0));
    assert_eq!(image.size, [2, 2], "the image follows the CURRENT extent");
    assert!(
        image
            .pixels
            .iter()
            .all(|pixel| *pixel == egui::Color32::TRANSPARENT),
        "no stale texel survives the shrink"
    );
}
