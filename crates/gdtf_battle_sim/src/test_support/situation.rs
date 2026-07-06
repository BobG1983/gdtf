//! The fluent [`SituationBuilder`] + the canonical [`fixtures`] — the shared way
//! every test assembles a [`Situation`], plus the named multi-ganger fixtures the
//! sim's own tests and the downstream crates' battle tests both reach for.

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

/// The GTW-414 gang NAME a test builder assigns a ganger to, derived from its faction
/// (`gang_{faction}`) — so the v2 split groups same-faction gangers into one reusable
/// gang roster while keeping per-faction rosters distinct. Test-only convention; shipped
/// content names its gangs by the file stem.
#[must_use]
fn gang_name_for(faction: Faction) -> GangName {
    GangName::new(format!("gang_{}", *faction))
}

/// The canonical test terrain definition UUIDs — the
/// [`TerrainUuid`](crate::terrain::def::TerrainUuid) keys authored test situations use so the
/// test registry (`test_terrain_registry()`) can resolve them (GTW-491: switched from
/// filename-stem strings to stable UUIDs).
///
/// These are NOT shipped to production assets — they live only in the test support layer and
/// resolve against a [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) built by
/// the test harness (see `test_terrain_registry` in `test_support/registries.rs`). The UUIDs
/// are fixed `from_u128` constants so a test can assert a resolved kind discriminatingly.
pub mod test_pieces {
    use bevy::asset::uuid::Uuid;

    use crate::terrain::def::TerrainUuid;

    /// The test WALL def UUID — a HIGH-band structural wall (`Wall` sim-kind).
    pub const WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0001));
    /// The test SLAB def UUID — a destructible slab (`Slab` sim-kind).
    pub const SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0002));
    /// The test COVER (scatter) def UUID — a LOW-band cover prop (`Cover` sim-kind).
    pub const COVER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0003));
    /// The test FLOOR def UUID — a walkable floor (`Slab` sim-kind in the GTW-491 model).
    pub const FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0004));
    /// The test VISION-SLAB def UUID (GTW-502) — a `Slab` sim-kind carrying an explicit
    /// `BlocksVision` tag (so it occludes LoS/FoV at HIGH despite being a slab — the
    /// tag-driven gap-closer). Used by the GTW-502 discriminating pair against [`SLAB`].
    pub const VISION_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0005));
    /// The test PATH-SLAB def UUID (GTW-502 C7 independence) — a `Slab` sim-kind carrying ONLY
    /// a `BlocksPathfinding` tag (and NOT `BlocksVision`): it blocks a path but does NOT
    /// occlude vision.
    pub const PATH_SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0006));
    /// The test LOW-VISION-COVER def UUID (GTW-502 height-awareness) — a `Cover` sim-kind at
    /// the LOW band (occludes a LOW sightline, a HIGH one clears it).
    pub const LOW_VISION_COVER: TerrainUuid =
        TerrainUuid::new(Uuid::from_u128(0x0149_1491_0000_0007));
}

/// An authored wall at `at` using the standard test wall def UUID — the terse cover helper
/// the fixtures + the per-crate tests use.
///
/// The piece resolves against the test terrain registry (see
/// `test_support/registries.rs` → `test_terrain_registry()`).
#[must_use]
pub const fn wall_at(at: CellLevel) -> CoverSpawn {
    CoverSpawn::new(at, test_pieces::WALL)
}

/// A fluent builder for a [`Situation`] — the canonical way every test assembles
/// a battlefield, replacing the hand-rolled `Situation { .. ..Situation::new() }`
/// struct-update literals. Starts empty (no gangers / cover / slabs / links,
/// `player_faction` = [`Faction::default`] = gang 0) and accretes via the `with_*`
/// / `*_at` methods.
///
/// GTW-414 schema v2: the builder accepts the ergonomic combined [`GangerSpawn`]
/// authoring records and SPLITS them on build — each becomes a
/// [`PlacedGanger`](crate::situation::PlacedGanger) (into the [`Situation`]) plus a
/// [`GangMember`](crate::ganger::GangMember) (into a synthesized [`GangRegistry`], keyed
/// by a per-faction [`GangName`] via `gang_name_for`). [`build`](SituationBuilder::build)
/// yields the [`Situation`] alone (for tests that don't run setup);
/// [`build_with_gangs`](SituationBuilder::build_with_gangs) yields the
/// `(Situation, GangRegistry)` pair the v2 [`setup_battle`](crate::situation::setup_battle)
/// resolves against.
#[derive(Debug, Clone, Default)]
pub struct SituationBuilder {
    /// The non-ganger situation fields (walls / scatter / slabs / links / player faction
    /// / floors / theme / `grid_size`) — accreted directly.
    situation: Situation,
    /// The combined ganger authoring records, accreted and split on build (GTW-414).
    gangers:   Vec<GangerSpawn>,
}

impl SituationBuilder {
    /// A fresh empty-situation builder (no gangers, cover, slabs, or links; player
    /// faction gang 0).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one authored ganger (a combined [`GangerSpawn`] record, split on build).
    #[must_use]
    pub fn with_ganger(mut self, ganger: GangerSpawn) -> Self {
        self.gangers.push(ganger);
        self
    }

    /// Append a batch of authored gangers (authored order preserved; split on build).
    #[must_use]
    pub fn with_gangers(mut self, gangers: impl IntoIterator<Item = GangerSpawn>) -> Self {
        self.gangers.extend(gangers);
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
    /// test slab def UUID. Resolves against the test terrain registry.
    #[must_use]
    pub fn slab_at(mut self, at: CellLevel) -> Self {
        self.situation
            .slabs
            .push(SlabSpawn::new(at, test_pieces::SLAB));
        self
    }

    /// Append a present slab at `(cell, level)` using an EXPLICIT terrain def UUID — for tests
    /// (e.g. GTW-502) that author a tagged slab (`VISION_SLAB` / `PATH_SLAB`) rather than the
    /// untagged [`slab_at`](SituationBuilder::slab_at) default. Resolves against the test
    /// terrain registry.
    #[must_use]
    pub fn slab_piece_at(mut self, at: CellLevel, piece: crate::terrain::def::TerrainUuid) -> Self {
        self.situation.slabs.push(SlabSpawn::new(at, piece));
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

    /// Consume the builder and yield the configured [`Situation`] alone — the v2
    /// placements (each combined ganger split into a
    /// [`PlacedGanger`](crate::situation::PlacedGanger)), discarding the synthesized
    /// [`GangRegistry`]. For tests that inspect the situation but don't run
    /// [`setup_battle`](crate::situation::setup_battle); use
    /// [`build_with_gangs`](SituationBuilder::build_with_gangs) when running a setup.
    #[must_use]
    pub fn build(self) -> Situation {
        self.build_with_gangs().0
    }

    /// Consume the builder and yield the `(Situation, GangRegistry)` pair (GTW-414): the
    /// v2 placements plus the synthesized gang registry the placements resolve against.
    ///
    /// Each accumulated combined [`GangerSpawn`] is `split` against a per-faction
    /// [`GangName`] (`gang_name_for`): its [`PlacedGanger`](crate::situation::PlacedGanger)
    /// goes into the situation (authored order preserved) and its
    /// [`GangMember`](crate::ganger::GangMember) into the gang's roster (deduped by member
    /// name — two same-faction gangers sharing a name share one roster member, which is
    /// fine since they then carry identical attributes).
    #[must_use]
    pub fn build_with_gangs(mut self) -> (Situation, GangRegistry) {
        let mut rosters: std::collections::BTreeMap<String, GangRoster> =
            std::collections::BTreeMap::new();
        for ganger in &self.gangers {
            let gang = gang_name_for(ganger.faction);
            let (placed, member) = ganger.split(gang.clone());
            self.situation.gangers.push(placed);
            let roster = rosters.entry((*gang).clone()).or_default();
            // Dedup by member name (the gang's roster is name-keyed at lookup): only the
            // first member of a given name is inserted; a same-name placement reuses it.
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

/// The canonical TEST [`GangRegistry`] — the union of every standard fixture's
/// synthesized gangs, so a harness that fields any of the canonical fixtures (or any
/// `ganger_at(_, f)`-built situation for `f ∈ {0, 1, 2}`) has the gangs its placements
/// resolve against (GTW-414).
///
/// Built from the fixtures via [`build_with_gangs`](SituationBuilder::build_with_gangs)
/// — the SAME split the situations use — so it can never drift from what
/// [`ganger_at`] produces. The downstream battle harnesses (`gdtf_test_utils`'s
/// `BattleAppBuilder`, the per-app / presenter / input integration tests) insert THIS
/// alongside [`test_weapon_registry`](super::registries::test_weapon_registry) /
/// [`test_armor_registry`](super::registries::test_armor_registry), exactly as they
/// already insert those, so `setup_battle` resolves every fielded ganger.
#[must_use]
pub fn test_gang_registry() -> GangRegistry {
    // Build one situation that fields every canonical test ganger, then take its
    // synthesized registry. Because every member of a gang is merged into ONE roster by
    // `build_with_gangs`, this single pass yields a registry holding all the gangs +
    // members any canonical fixture references — no manual unioning, no drift from
    // `ganger_at` (the SAME split produces it). Two member families are covered, each on
    // EVERY canonical faction (0 / 1 / 2):
    //
    // - "Test Ganger" — the DEFAULT `GangerSpawnBuilder` name (faction-independent), which
    //   harnesses field on either side via `.faction(_)` (e.g. the battle-bootstrap shooter
    //   / target). It must therefore exist in gang_0/gang_1/gang_2, not just gang_0.
    // - "Ganger {f}" — what `ganger_at(_, f)` produces (one per faction).
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new().faction(Faction::new(0)).build(), // gang_0 ⇐ "Test Ganger"
            GangerSpawnBuilder::new().faction(Faction::new(1)).build(), // gang_1 ⇐ "Test Ganger"
            GangerSpawnBuilder::new().faction(Faction::new(2)).build(), // gang_2 ⇐ "Test Ganger"
            ganger_at(key(0, 0, 0), 0),                                 // gang_0 ⇐ "Ganger 0"
            ganger_at(key(0, 0, 0), 1),                                 // gang_1 ⇐ "Ganger 1"
            ganger_at(key(0, 0, 0), 2),                                 // gang_2 ⇐ "Ganger 2"
        ])
        .build_with_gangs()
        .1
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
