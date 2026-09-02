//! Situation builders and fixtures for tests.

use super::{
    ganger::{GangerSpawnBuilder, ganger_at},
    registries::key,
};
use crate::{
    ganger::{Faction, GangName, GangRegistry, GangRoster},
    level::GridSize,
    metric::CellLevel,
    situation::{CoverSpawn, FloorSpawn, GangerSpawn, Situation, SlabSpawn},
    terrain::{def::TerrainUuid, facing::TerrainFacing},
    vertical::VerticalLink,
};

#[must_use]
fn gang_name_for(faction: Faction) -> GangName {
    GangName::new(format!("gang_{}", *faction))
}

/// Stable terrain UUIDs used by test fixtures.
pub mod test_pieces {
    use bevy::asset::uuid::Uuid;

    use crate::terrain::def::TerrainUuid;

    /// Wall piece.
    pub const WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0001));
    /// Slab piece.
    pub const SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0002));
    /// Cover piece.
    pub const COVER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0003));
    /// Floor piece.
    pub const FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0004));
    /// Vision-blocking slab.
    pub const VISION_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0005));
    /// Path-blocking slab.
    pub const PATH_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0006));
    /// Low vision cover.
    pub const LOW_VISION_COVER: TerrainUuid =
        TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0007));
    /// Emplacement piece.
    pub const EMPLACEMENT: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0008));
    /// Emplacement piece that leaves a blocking wall behind when it is destroyed.
    pub const EMPLACEMENT_LEAVING_WALL: TerrainUuid =
        TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0009));
    /// Openable wall piece with shut and open art on every facing.
    pub const DOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_000A));
    /// Wall piece whose four edge views each name a different sprite key.
    pub const FACING_WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_000B));
    /// Slab piece tagged as a staircase, with the climb art on every facing.
    pub const STAIR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_000C));
    /// Cover piece whose art is the rubble sprite, told apart from [`COVER`].
    pub const RUBBLE_COVER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_000D));
    /// Slab piece a destroyed cover leaves behind, drawn with its own floor sprite.
    pub const SUCCESSOR_FLOOR: TerrainUuid =
        TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_000E));
    /// Slab piece whose one view names a sprite key no sprite def holds.
    pub const UNRESOLVABLE: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_000F));
}

/// Wall spawn at a cell.
#[must_use]
pub const fn wall_at(at: CellLevel) -> CoverSpawn {
    CoverSpawn::new(at, test_pieces::WALL, TerrainFacing::North)
}

/// Emplacement spawn at a cell.
#[must_use]
pub const fn emplacement_at(at: CellLevel) -> CoverSpawn {
    CoverSpawn::new(at, test_pieces::EMPLACEMENT, TerrainFacing::North)
}

/// Fluent builder for [`Situation`] and optional gang registry.
#[derive(Debug, Clone, Default)]
pub struct SituationBuilder {
    situation: Situation,
    gangers:   Vec<GangerSpawn>,
}

impl SituationBuilder {
    /// Empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one ganger spawn.
    #[must_use]
    pub fn with_ganger(mut self, ganger: GangerSpawn) -> Self {
        self.gangers.push(ganger);
        self
    }

    /// Add several ganger spawns.
    #[must_use]
    pub fn with_gangers(mut self, gangers: impl IntoIterator<Item = GangerSpawn>) -> Self {
        self.gangers.extend(gangers);
        self
    }

    /// Place a wall at a cell.
    #[must_use]
    pub fn wall_at(mut self, at: CellLevel) -> Self {
        self.situation.walls.push(wall_at(at));
        self
    }

    /// Add scatter cover.
    #[must_use]
    pub fn with_scatter(mut self, cover: CoverSpawn) -> Self {
        self.situation.scatter.push(cover);
        self
    }

    /// Place a slab at a cell.
    #[must_use]
    pub fn slab_at(mut self, at: CellLevel) -> Self {
        self.situation
            .slabs
            .push(SlabSpawn::new(at, test_pieces::SLAB, TerrainFacing::North));
        self
    }

    /// Place a specific slab piece at a cell.
    #[must_use]
    pub fn slab_piece_at(mut self, at: CellLevel, piece: crate::terrain::def::TerrainUuid) -> Self {
        self.situation
            .slabs
            .push(SlabSpawn::new(at, piece, TerrainFacing::North));
        self
    }

    /// Set the terrain piece every cell with no authored floor takes.
    #[must_use]
    pub const fn default_floor(mut self, piece: TerrainUuid) -> Self {
        self.situation.default_floor = piece;
        self
    }

    /// Author a per-cell floor override.
    #[must_use]
    pub fn floor_at(mut self, at: CellLevel, piece: TerrainUuid, facing: TerrainFacing) -> Self {
        self.situation
            .floors
            .push(FloorSpawn::new(at, piece, facing));
        self
    }

    /// Set the authored board size.
    #[must_use]
    pub const fn grid_size(mut self, size: GridSize) -> Self {
        self.situation.grid_size = size;
        self
    }

    /// Add a vertical link.
    #[must_use]
    pub fn vertical_link(mut self, link: VerticalLink) -> Self {
        self.situation.vertical_links.push(link);
        self
    }

    /// Set the player faction.
    #[must_use]
    pub const fn player_faction(mut self, faction: Faction) -> Self {
        self.situation.player_faction = faction;
        self
    }

    /// Build the situation only.
    #[must_use]
    pub fn build(self) -> Situation {
        self.build_with_gangs().0
    }

    /// Build the situation and a matching gang registry.
    #[must_use]
    pub fn build_with_gangs(mut self) -> (Situation, GangRegistry) {
        let mut rosters: std::collections::BTreeMap<String, GangRoster> =
            std::collections::BTreeMap::new();
        for ganger in &self.gangers {
            let gang = gang_name_for(ganger.faction);
            let (placed, member) = ganger.split(gang.clone());
            self.situation.gangers.push(placed);
            let roster = rosters.entry((*gang).clone()).or_default();
            if roster.member(&member.name).is_none() {
                roster.members.push(member);
            }
        }
        let registry = GangRegistry::new(
            rosters
                .into_iter()
                .map(|(name, roster)| (GangName::new(name), roster)),
        );
        (self.situation, registry)
    }
}

/// Registry with a few default test gangs.
#[must_use]
pub fn test_gang_registry() -> GangRegistry {
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new().faction(Faction::new(0)).build(),
            GangerSpawnBuilder::new().faction(Faction::new(1)).build(),
            GangerSpawnBuilder::new().faction(Faction::new(2)).build(),
            ganger_at(key(0, 0, 0), 0),
            ganger_at(key(0, 0, 0), 1),
            ganger_at(key(0, 0, 0), 2),
        ])
        .build_with_gangs()
        .1
}

/// Ready-made situations for common test cases.
pub mod fixtures {
    use super::{Faction, Situation, SituationBuilder, ganger_at, key};

    /// Two gangers, factions 0 and 1.
    #[must_use]
    pub fn two_ganger() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .build()
    }

    /// One player and two enemies.
    #[must_use]
    pub fn one_player_two_enemies() -> Situation {
        SituationBuilder::new()
            .with_gangers([
                ganger_at(key(5, 6, 0), 0),
                ganger_at(key(7, 8, 0), 1),
                ganger_at(key(9, 10, 0), 1),
            ])
            .build()
    }

    /// Three player gangers and one enemy, so a selection cycle can be told from its reverse.
    ///
    /// Two gangers make the cycle direction-blind: forward and back land on the same one.
    #[must_use]
    pub fn three_player_gangers() -> Situation {
        SituationBuilder::new()
            .with_gangers([
                ganger_at(key(5, 6, 0), 0),
                ganger_at(key(6, 6, 0), 0),
                ganger_at(key(7, 6, 0), 0),
                ganger_at(key(12, 12, 0), 1),
            ])
            .build()
    }

    /// Only player-faction gangers.
    #[must_use]
    pub fn player_only() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 0)])
            .build()
    }

    /// Minimal map with a wall and a slab.
    #[must_use]
    pub fn minimal_with_cells() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .wall_at(key(1, 2, 0))
            .slab_at(key(3, 4, 1))
            .build()
    }

    /// Two gangers with player faction set to 1.
    #[must_use]
    pub fn two_ganger_player_faction_one() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .player_faction(Faction::new(1))
            .build()
    }
}
