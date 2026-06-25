//! The fluent [`SituationBuilder`] + the canonical [`fixtures`] — the shared way
//! every test assembles a [`Situation`], plus the named multi-ganger fixtures the
//! sim's own tests and the downstream crates' battle tests both reach for.

use super::{ganger::ganger_at, registries::key};
use crate::{
    ganger::Faction,
    metric::CellLevel,
    situation::{CoverSpawn, GangerSpawn, Situation, SlabSpawn},
    terrain::piece::TerrainName,
    vertical::VerticalLink,
};

/// The canonical test terrain piece names — the string keys authored test
/// situations use so the test registry (`test_terrain_registry()`) can resolve them.
///
/// These are NOT shipped to production assets — they live only in the test
/// support layer and resolve against a registry built by the test harness
/// (see `test_terrain_registry` in `test_support/registries.rs`).
pub mod test_pieces {
    /// The test wall piece key — a HIGH-band structural wall with hp=120, prot=8, hard=4.
    pub const WALL: &str = "test-wall";
    /// The test slab piece key — a slab with hp=120, prot=4, hard=2.
    pub const SLAB: &str = "test-slab";
    /// The test cover (scatter) piece key — a LOW-band cover with hp=30, prot=2, hard=1.
    pub const COVER: &str = "test-cover";
    /// The test floor piece key — a floor with `move_cost=4` (the A* admissibility
    /// minimum).
    pub const FLOOR: &str = "test-floor";
}

/// An authored wall at `at` using the standard test wall piece key — the terse
/// cover helper the fixtures + the per-crate tests use.
///
/// The piece resolves against the test terrain registry (see
/// `test_support/registries.rs` → `test_terrain_registry()`).
#[must_use]
pub fn wall_at(at: CellLevel) -> CoverSpawn {
    CoverSpawn::new(at, TerrainName::new(test_pieces::WALL.to_owned()))
}

/// A fluent builder for a [`Situation`] — the canonical way every test assembles
/// a battlefield, replacing the hand-rolled `Situation { .. ..Situation::new() }`
/// struct-update literals. Starts empty (no gangers / cover / slabs / links,
/// `player_faction` = [`Faction::default`] = gang 0) and accretes via the `with_*`
/// / `*_at` methods; [`build`](SituationBuilder::build) yields the [`Situation`].
#[derive(Debug, Clone, Default)]
pub struct SituationBuilder {
    situation: Situation,
}

impl SituationBuilder {
    /// A fresh empty-situation builder (no gangers, cover, slabs, or links; player
    /// faction gang 0).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one authored ganger.
    #[must_use]
    pub fn with_ganger(mut self, ganger: GangerSpawn) -> Self {
        self.situation.gangers.push(ganger);
        self
    }

    /// Append a batch of authored gangers (authored order preserved).
    #[must_use]
    pub fn with_gangers(mut self, gangers: impl IntoIterator<Item = GangerSpawn>) -> Self {
        self.situation.gangers.extend(gangers);
        self
    }

    /// Append a [`wall_at`] wall at `(cell, level)`.
    #[must_use]
    pub fn wall_at(mut self, at: CellLevel) -> Self {
        self.situation.walls.push(wall_at(at));
        self
    }

    /// Append one authored scatter / prop piece (a [`CoverSpawn`] — the same
    /// schema as a wall, into the `scatter` list) — for tests that author
    /// destructible cover with explicit piece keys rather than the [`wall_at`] default.
    #[must_use]
    pub fn with_scatter(mut self, cover: CoverSpawn) -> Self {
        self.situation.scatter.push(cover);
        self
    }

    /// Append a present floor / roof slab at `(cell, level)` using the standard
    /// test slab piece key (`"test-slab"`). Resolves against the test terrain registry.
    #[must_use]
    pub fn slab_at(mut self, at: CellLevel) -> Self {
        self.situation.slabs.push(SlabSpawn::new(
            at,
            TerrainName::new(test_pieces::SLAB.to_owned()),
        ));
        self
    }

    /// Append an authored [`VerticalLink`] (a stair / ladder between storeys) —
    /// the builder mirror of the [`slab_at`](SituationBuilder::slab_at) /
    /// [`wall_at`](SituationBuilder::wall_at) terrain methods, for tests that
    /// author vertical connectivity (valid links AND deliberately-bad links the
    /// validation-abort tests reach for).
    #[must_use]
    pub fn vertical_link(mut self, link: VerticalLink) -> Self {
        self.situation.vertical_links.push(link);
        self
    }

    /// Set the gang the human player controls (every other faction is the enemy).
    #[must_use]
    pub const fn player_faction(mut self, faction: Faction) -> Self {
        self.situation.player_faction = faction;
        self
    }

    /// Consume the builder and yield the configured [`Situation`].
    #[must_use]
    pub fn build(self) -> Situation {
        self.situation
    }
}

/// The canonical named [`Situation`] fixtures — the shared multi-ganger
/// battlefields the sim's own tests and the downstream crates' battle tests both
/// reach for, built over [`SituationBuilder`] + [`ganger_at`].
pub mod fixtures {
    use super::{Faction, Situation, SituationBuilder, ganger_at, key};

    /// A valid two-ganger fixture (no cover / slabs / links — link-free validates
    /// trivially): faction 0 at `(5, 6, 0)` and faction 1 at `(7, 8, 0)`. The
    /// `player_faction` defaults to gang 0.
    #[must_use]
    pub fn two_ganger() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .build()
    }

    /// A three-ganger fixture: one player ganger (faction 0) + two enemy gangers
    /// (faction 1) — the win-side fixture (`player_faction` defaults to gang 0).
    /// Link-free, so a setup validates trivially.
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

    /// A player-only fixture: every ganger is faction 0 — the degenerate /
    /// empty-enemy-roster fixture (no enemy of the player faction, so a victory
    /// census never wins).
    #[must_use]
    pub fn player_only() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 0)])
            .build()
    }

    /// A two-ganger fixture with one wall and one upper-storey slab — the minimal
    /// fixture with cover + a slab cell, for tests that need authored terrain
    /// alongside gangers. Faction 0 at `(5, 6, 0)`, faction 1 at `(7, 8, 0)`, a
    /// wall at `(1, 2, 0)`, a slab at `(3, 4, 1)`.
    #[must_use]
    pub fn minimal_with_cells() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .wall_at(key(1, 2, 0))
            .slab_at(key(3, 4, 1))
            .build()
    }

    /// The two-ganger fixture with `player_faction` authored to gang 1 (overriding
    /// the gang-0 default) — proving a seed reads `situation.player_faction`, not a
    /// hardcoded gang 0.
    #[must_use]
    pub fn two_ganger_player_faction_one() -> Situation {
        SituationBuilder::new()
            .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
            .player_faction(Faction::new(1))
            .build()
    }
}
