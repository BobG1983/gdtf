//! [`Situation`] — the canonical authored battlefield aggregate plus its
//! authored-cells iterator.

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{
    piece_spawns::{CoverSpawn, FieldSpawn, FloorSpawn, SlabSpawn},
    placed_ganger::PlacedGanger,
    roster_member::RosterMember,
};
use crate::{
    ganger::Faction,
    level::{GridSize, ThemeUuid},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    vertical::VerticalLink,
};

/// derives. `#[serde(default)]` on each list lets an authored file omit a section it
#[derive(Debug, Clone, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct Situation {
                                                pub gangers:        Vec<PlacedGanger>,
                                        /// cells). The struct-level `#[serde(default)]` gives an empty list, so every
            pub rosters:        Vec<RosterMember>,
                            /// `theme_uuid` field — the canonical sim theme is now this one `theme`). `#[serde(default)]`
        /// field. An authored `theme: "<uuid>"` parses the `#[serde(transparent)]` [`ThemeUuid`]
        pub theme:          ThemeUuid,
                    /// `#[serde(default)]` supplies [`GridSize::default`] = the FULL documented `60×60×8`
                        pub grid_size:      GridSize,
        pub walls:          Vec<CoverSpawn>,
        pub scatter:        Vec<CoverSpawn>,
                    pub slabs:          Vec<SlabSpawn>,
                pub vertical_links: Vec<VerticalLink>,
                /// `#[serde(default)]` supplies [`Faction::default`] = `Faction(0)` for any
                /// ([`Faction`] is `#[serde(transparent)]`).
    pub player_faction: Faction,
                            /// `#[serde(default)]` supplies the NIL sentinel ([`TerrainUuid::default`] —
                        pub default_floor:  TerrainUuid,
            /// `#[serde(default)]` gives an empty list (the common case: uniform floor).
    pub floors:         Vec<FloorSpawn>,
                        /// terrain). `#[serde(default)]` gives an empty list, so every EXISTING situation `.ron`
        pub fields:         Vec<FieldSpawn>,
}

impl Situation {
        #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

                                        pub fn authored_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.walls
            .iter()
            .map(|c| c.at)
            .chain(self.scatter.iter().map(|c| c.at))
            .chain(self.slabs.iter().map(|s| s.at))
    }
}
