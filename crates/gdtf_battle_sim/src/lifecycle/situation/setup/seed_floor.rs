use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Commands, Deref},
};

use super::super::terrain_resolve::resolve_slab_def;
use crate::{
    metric::{Cell, CellLevel, CellUnit, Level},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    situation::Situation,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainCell,
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
};

// The one storey the drawn floor field covers.
const GROUND: Level = Level::new(0);

// The def and facing a cell takes: its own storey-0 `FloorSpawn`, else the default floor.
fn floor_for(situation: &Situation, at: CellLevel) -> (TerrainUuid, TerrainFacing) {
    situation.floors.iter().find(|floor| floor.at == at).map_or(
        (situation.default_floor, TerrainFacing::default()),
        |floor| (floor.piece, floor.facing),
    )
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

// The graphic a floor uuid draws, resolved once per uuid so a refused def logs once.
fn graphic_for<'a>(
    seen: &'a mut HashMap<TerrainUuid, Option<TerrainGraphicKey>>,
    registry: &TerrainDefRegistry,
    piece: TerrainUuid,
) -> Option<&'a TerrainGraphicKey> {
    seen.entry(piece)
        .or_insert_with(|| {
            let def = registry.def(&piece)?;
            resolve_slab_def(&piece, def).map(|resolved| resolved.graphic)
        })
        .as_ref()
}

/// Spawn a floor piece on every storey-0 cell of the drawn extent that holds no
/// authored cover, scatter or slab.
pub(super) fn seed_floor_terrain(
    situation: &Situation,
    terrain: Option<&TerrainDefRegistry>,
    commands: &mut Commands,
) {
    let Some(registry) = terrain else {
        return;
    };
    let taken: HashSet<CellLevel> = situation.authored_cells().collect();
    let mut seen: HashMap<TerrainUuid, Option<TerrainGraphicKey>> = HashMap::new();
    let width = axis_bound(GridExtent::new(GRID_WIDTH));
    let height = axis_bound(GridExtent::new(GRID_HEIGHT));
    for y in 0..*height {
        for x in 0..*width {
            let at = CellLevel::new(Cell::new(x, y), GROUND);
            if taken.contains(&at) {
                continue;
            }
            let (piece, facing) = floor_for(situation, at);
            let Some(graphic) = graphic_for(&mut seen, registry, piece).cloned() else {
                continue;
            };
            commands.spawn((TerrainCell::new(at), piece, facing, graphic));
        }
    }
}
