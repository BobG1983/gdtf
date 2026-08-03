use super::{
    ganger::{GangerSpawnBuilder, ganger_at},
    registries::key,
};
use crate::{
    ganger::{Faction, GangName, GangRegistry, GangRoster},
    metric::CellLevel,
    situation::{CoverSpawn, GangerSpawn, Situation, SlabSpawn},
    vertical::VerticalLink,
};

#[must_use]
fn gang_name_for(faction: Faction) -> GangName {
    GangName::new(format!("gang_{}", *faction))
}

pub mod test_pieces {
    use bevy::asset::uuid::Uuid;

    use crate::terrain::def::TerrainUuid;

        pub const WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0001));
        pub const SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0002));
        pub const COVER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0003));
        pub const FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0004));
                pub const VISION_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0005));
                pub const PATH_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0006));
            pub const LOW_VISION_COVER: TerrainUuid =
        TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0007));
}

#[must_use]
pub const fn wall_at(at: CellLevel) -> CoverSpawn {
    CoverSpawn::new(at, test_pieces::WALL)
}

#[derive(Debug, Clone, Default)]
pub struct SituationBuilder {
            situation: Situation,
        gangers:   Vec<GangerSpawn>,
}

impl SituationBuilder {
            #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

        #[must_use]
    pub fn with_ganger(mut self, ganger: GangerSpawn) -> Self {
        self.gangers.push(ganger);
        self
    }

        #[must_use]
    pub fn with_gangers(mut self, gangers: impl IntoIterator<Item = GangerSpawn>) -> Self {
        self.gangers.extend(gangers);
        self
    }

        #[must_use]
    pub fn wall_at(mut self, at: CellLevel) -> Self {
        self.situation.walls.push(wall_at(at));
        self
    }

                #[must_use]
    pub fn with_scatter(mut self, cover: CoverSpawn) -> Self {
        self.situation.scatter.push(cover);
        self
    }

            #[must_use]
    pub fn slab_at(mut self, at: CellLevel) -> Self {
        self.situation
            .slabs
            .push(SlabSpawn::new(at, test_pieces::SLAB));
        self
    }

                    #[must_use]
    pub fn slab_piece_at(mut self, at: CellLevel, piece: crate::terrain::def::TerrainUuid) -> Self {
        self.situation.slabs.push(SlabSpawn::new(at, piece));
        self
    }

                        #[must_use]
    pub fn vertical_link(mut self, link: VerticalLink) -> Self {
        self.situation.vertical_links.push(link);
        self
    }

        #[must_use]
    pub const fn player_faction(mut self, faction: Faction) -> Self {
        self.situation.player_faction = faction;
        self
    }

                            #[must_use]
    pub fn build(self) -> Situation {
        self.build_with_gangs().0
    }

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

pub mod fixtures {
    use super::{Faction, Situation, SituationBuilder, ganger_at, key};

                #[must_use]
    pub fn two_ganger() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .build()
    }

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

                #[must_use]
    pub fn player_only() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 0)])
            .build()
    }

                    #[must_use]
    pub fn minimal_with_cells() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .wall_at(key(1, 2, 0))
            .slab_at(key(3, 4, 1))
            .build()
    }

                #[must_use]
    pub fn two_ganger_player_faction_one() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .player_faction(Faction::new(1))
            .build()
    }
}
