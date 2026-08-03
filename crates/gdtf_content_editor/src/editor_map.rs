//! In-memory painted map model for the content editor.

use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{
    level::GridSize,
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::TerrainUuid,
};

pub(crate) const GROUND_LEVEL: Level = Level::new(0);

/// Sparse store of painted terrain tiles by cell-level.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorMap {
    painted: HashMap<CellLevel, TerrainUuid>,
}

impl EditorMap {
    /// Empty map with nothing painted.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            painted: HashMap::new(),
        }
    }

    /// Paint `tile` at ground level for `cell` if in bounds.
    pub fn paint(&mut self, cell: Cell, tile: TerrainUuid, size: GridSize) -> bool {
        self.paint_at(CellLevel::new(cell, GROUND_LEVEL), tile, size)
    }

    /// Paint `tile` at `slot` if in bounds.
    pub fn paint_at(&mut self, slot: CellLevel, tile: TerrainUuid, size: GridSize) -> bool {
        if !slot_in_bounds(slot, size) {
            return false;
        }
        self.painted.insert(slot, tile);
        true
    }

    /// Remove the tile at `slot`, returning the previous value.
    pub fn clear(&mut self, slot: CellLevel) -> Option<TerrainUuid> {
        self.painted.remove(&slot)
    }

    /// Tile at ground level for `cell`, if any.
    #[must_use]
    pub fn tile_at(&self, cell: Cell) -> Option<TerrainUuid> {
        self.tile_at_level(CellLevel::new(cell, GROUND_LEVEL))
    }

    /// Tile at `slot`, if any.
    #[must_use]
    pub fn tile_at_level(&self, slot: CellLevel) -> Option<TerrainUuid> {
        self.painted.get(&slot).copied()
    }

    /// Iterate all painted slots.
    pub fn painted(&self) -> impl Iterator<Item = (&CellLevel, &TerrainUuid)> {
        self.painted.iter()
    }

    /// Number of painted slots.
    #[must_use]
    pub fn painted_count(&self) -> usize {
        self.painted.len()
    }
}

fn slot_in_bounds(slot: CellLevel, size: GridSize) -> bool {
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    let levels = i32::from(*size.levels());
    slot.x >= 0
        && slot.x < width
        && slot.y >= 0
        && slot.y < height
        && slot.z >= 0
        && slot.z < levels
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        level::{GridHeight, GridLevels, GridSize, GridWidth},
        metric::{CellLevel, Level},
        prelude::Cell,
        terrain::def::TerrainUuid,
    };

    use super::EditorMap;

    fn small_size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(2))
            .unwrap_or_else(|_| GridSize::default())
    }

    const fn key(n: u128) -> TerrainUuid {
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
    }

    #[test]
    fn in_bounds_paint_is_recorded_and_read_back() {
        let mut map = EditorMap::new();
        assert_eq!(
            map.painted_count(),
            0,
            "a fresh model holds nothing painted"
        );
        assert!(
            map.tile_at(Cell::new(1, 1)).is_none(),
            "unpainted cell reads None"
        );

        let recorded = map.paint(Cell::new(1, 2), key(0xAA), small_size());
        assert!(recorded, "an in-bounds paint must be recorded (C2)");
        assert_eq!(map.painted_count(), 1, "one paint -> one entry");
        assert_eq!(
            map.tile_at(Cell::new(1, 2)),
            Some(key(0xAA)),
            "the painted cell reads back its tile keyed by its Cell (C2)",
        );
    }

    #[test]
    fn repaint_overwrites_the_same_cell() {
        let mut map = EditorMap::new();
        let cell = Cell::new(0, 0);
        assert!(map.paint(cell, key(0x1), small_size()));
        assert!(map.paint(cell, key(0x2), small_size()));
        assert_eq!(
            map.painted_count(),
            1,
            "repainting a cell must not add an entry"
        );
        assert_eq!(
            map.tile_at(cell),
            Some(key(0x2)),
            "repainting must overwrite with the latest tile",
        );
    }

    #[test]
    fn out_of_bounds_paint_is_clamped_out() {
        let mut map = EditorMap::new();
        let size = small_size();
        let width = i32::from(*size.width());
        let height = i32::from(*size.height());
        for out in [
            Cell::new(-1, 0),
            Cell::new(0, -1),
            Cell::new(width, 0),
            Cell::new(0, height),
            Cell::new(width, height),
        ] {
            assert!(
                !map.paint(out, key(0x9), size),
                "out-of-bounds cell {out:?} must be rejected (C3)",
            );
        }
        assert_eq!(
            map.painted_count(),
            0,
            "no out-of-bounds cell may enter the model (C3)"
        );
    }

    #[test]
    fn level_aware_paint_is_storey_scoped_and_clamped() {
        let mut map = EditorMap::new();
        let size = small_size();
        let ground = CellLevel::new(Cell::new(1, 1), Level::new(0));
        let above = CellLevel::new(Cell::new(1, 1), Level::new(1));

        assert!(map.paint_at(ground, key(0x10), size), "L0 paint records");
        assert!(map.paint_at(above, key(0x20), size), "L1 paint records");
        assert_eq!(
            map.painted_count(),
            2,
            "the same x/y on two storeys are two distinct slots",
        );
        assert_eq!(map.tile_at_level(ground), Some(key(0x10)));
        assert_eq!(map.tile_at_level(above), Some(key(0x20)));

        let over = CellLevel::new(Cell::new(1, 1), Level::new(2));
        assert!(
            !map.paint_at(over, key(0x30), size),
            "L2 is past the extent"
        );
        assert_eq!(
            map.painted_count(),
            2,
            "no over-extent storey enters the model"
        );

        assert_eq!(
            map.clear(above),
            Some(key(0x20)),
            "clear returns the removed tile"
        );
        assert_eq!(map.painted_count(), 1, "clear removes one entry");
        assert!(
            map.tile_at_level(above).is_none(),
            "the cleared slot reads None"
        );
        assert!(
            map.clear(above).is_none(),
            "clearing an empty slot is a no-op"
        );
    }
}
