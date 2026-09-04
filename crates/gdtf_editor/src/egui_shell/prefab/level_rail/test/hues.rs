use gdtf_battle_sim::{
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::facing::TerrainFacing,
};

use super::support::{DOOR, STAIR, UNKNOWN, WALL, registry, size, texel};
use crate::{
    editor_map::EditorMap,
    egui_shell::prefab::level_rail::occupancy::{FALLBACK_HUE, storey_image},
};

#[test]
fn the_rail_tells_a_wall_a_door_and_a_stair_apart() {
    let size = size();
    let mut map = EditorMap::new();
    let north = TerrainFacing::default();
    let ground = Level::new(0);
    for (x, tile) in [(0, WALL), (1, DOOR), (2, STAIR), (3, UNKNOWN)] {
        assert!(
            map.paint_at(CellLevel::new(Cell::new(x, 0), ground), tile, north, size),
            "painting {tile:?} at ({x}, 0) on the ground storey must be legal",
        );
    }
    let reg = registry();

    let image = storey_image(&map, Some(&reg), size, ground);
    let wall = texel(&image, 0, 0);
    let door = texel(&image, 1, 0);
    let stair = texel(&image, 2, 0);
    assert_ne!(
        wall, door,
        "a wall and a door share TerrainSimKind::Wall, so a hue read off the kind alone would \
         draw them the same and the rail would stop naming the door",
    );
    assert_ne!(
        wall, stair,
        "a wall and a stair must not share a hue on the rail",
    );
    assert_ne!(
        door, stair,
        "a door and a stair must not share a hue on the rail",
    );
    assert_eq!(
        texel(&image, 3, 0),
        FALLBACK_HUE,
        "a key the registry holds no def for still draws, in the fallback hue",
    );
}
