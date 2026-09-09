use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Commands, Deref},
};

use super::super::terrain_resolve::resolve_slab_def;
use crate::{
    metric::{Cell, CellLevel, CellUnit, Level},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    situation::BattleMap,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainCell,
        facing::TerrainFacing,
    },
};

// The one storey the drawn floor field covers.
const GROUND: Level = Level::new(0);

// The def and facing a cell takes: its own storey-0 `FloorSpawn`, else the default floor.
fn floor_for(map: &BattleMap, at: CellLevel) -> (TerrainUuid, TerrainFacing) {
    map.floors
        .iter()
        .find(|floor| floor.at == at)
        .map_or((map.default_floor, TerrainFacing::default()), |floor| {
            (floor.piece, floor.facing)
        })
}

// One dimension of the dense grid, in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct GridExtent(usize);

impl GridExtent {
    #[must_use]
    const fn new(extent: usize) -> Self {
        Self(extent)
    }
}

// The cell axis bound a grid extent reaches.
fn axis_bound(extent: GridExtent) -> CellUnit {
    CellUnit::new(i32::try_from(*extent).unwrap_or(i32::MAX))
}

// Whether a floor uuid stands a piece up: the registry holds it and it is a Slab.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct FloorStands(bool);

impl FloorStands {
    #[must_use]
    const fn new(stands: bool) -> Self {
        Self(stands)
    }
}

// Whether a floor uuid resolves, answered once per uuid so a refused def logs once.
fn floor_stands(
    seen: &mut HashMap<TerrainUuid, FloorStands>,
    registry: &TerrainDefRegistry,
    piece: TerrainUuid,
) -> FloorStands {
    *seen.entry(piece).or_insert_with(|| {
        let stands = registry
            .def(&piece)
            .is_some_and(|def| resolve_slab_def(&piece, def).is_some());
        FloorStands::new(stands)
    })
}

/// Spawn a floor piece on every storey-0 cell of the drawn extent that holds no
/// authored cover, scatter or slab.
pub(super) fn seed_floor_terrain(
    map: &BattleMap,
    terrain: Option<&TerrainDefRegistry>,
    commands: &mut Commands,
) {
    let Some(registry) = terrain else {
        return;
    };
    let taken: HashSet<CellLevel> = map.authored_cells().collect();
    let mut seen: HashMap<TerrainUuid, FloorStands> = HashMap::new();
    let width = axis_bound(GridExtent::new(GRID_WIDTH));
    let height = axis_bound(GridExtent::new(GRID_HEIGHT));
    for y in 0..*height {
        for x in 0..*width {
            let at = CellLevel::new(Cell::new(x, y), GROUND);
            if taken.contains(&at) {
                continue;
            }
            let (piece, facing) = floor_for(map, at);
            if !*floor_stands(&mut seen, registry, piece) {
                continue;
            }
            commands.spawn((TerrainCell::new(at), piece, facing));
        }
    }
}
