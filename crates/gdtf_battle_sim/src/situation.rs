//! The canonical authored **situation** and the setup that pours it into the
//! battle ECS — the E1.8 situation→entities slice. The setup systems here build the
//! scene from the situation (floor, walls, scatter, gangers); the situation is
//! gangers, walls, scatter, upper-floor slabs, and stair/ladder vertical links —
//! every placement an optional storey, position always the pair (cell, level). This
//! module is the in-crate **setup-on-entry source of truth** for that construction.
//!
//! [`Situation`] is the **one canonical** authored battlefield value — it
//! supersedes the GTW-156 placeholder (renamed to
//! [`crate::occupancy::OccupancyInput`], the grid-relevant construction input) and
//! carries every authored field:
//!
//! - **gangers** ([`GangerSpawn`]): each a `(cell, level)` plus the E1.2 component
//!   VALUES ([`Faction`] / [`Facing`] / [`Stance`] / [`Aiming`] / [`Hp`] /
//!   [`Wounds`] / [`Tu`] / [`LifeState`]), the E3.0 attribute stats
//!   ([`Shooting`](crate::ganger::Shooting) / [`Toughness`](crate::ganger::Toughness)
//!   / [`Luck`](crate::ganger::Luck)), and a read-only [`SourceArmor`] record
//!   (`armor_by_part`) to seed the battle-local [`WornArmor`] (E1.3).
//! - **walls** + **scatter/props** ([`CoverSpawn`], the same schema for both): each
//!   a `(cell, level)`, a [`TerrainKind`], the cover's max [`CoverHp`], its
//!   [`HeightBand`], and its armor stats ([`ArmorProtection`] / [`ArmorHardness`]).
//! - **slabs** (a `Vec<`[`CellLevel`]`>`): the `(cell, level)`s that carry a present
//!   floor / roof slab.
//! - **`vertical_links`** (a `Vec<`[`VerticalLink`]`>`, moved here from the GTW-156
//!   placeholder): the authored stair / ladder links (E1.10).
//!
//! [`setup_battle`] reads a [`Situation`] and builds the battle in the ECS world
//! (the setup systems described above). It is **render-free** and driven from
//! a headless `MinimalPlugins` app (it takes only [`Commands`] — no renderer, no
//! asset server). For each ganger it `commands.spawn(...)`s ALL the per-field
//! components plus the seeded [`WornArmor`], capturing the returned Bevy [`Entity`](bevy::prelude::Entity)
//! handle — **never a numeric id** (GTW-10 / GTW-12). It then seeds the
//! [`CoverLedger`] (E1.4) from the walls + scatter, the [`SurfaceGrid`] (E1.5) from
//! the slabs, and the [`OccupancyGrid`] (E1.6) from the authored terrain plus the
//! SPAWNED occupant entities (via [`OccupancyGrid::build_from_occupancy_input`]),
//! inserting each as a resource. Finally it validates and inserts the
//! [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) (E1.10), returning [`InvalidVerticalLink`] if an authored
//! link is bad (the no-panic contract).

use bevy::{platform::collections::HashSet, prelude::Commands, reflect::TypePath};
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection, SourceArmor, WornArmor},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{
        Aiming, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stance, Toughness, Tu,
        Wounds,
    },
    metric::CellLevel,
    occupancy::{OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainKind, TerrainPlacement},
    surface::{SlabState, SurfaceGrid},
    vertical::{InvalidVerticalLink, VerticalLink, build_vertical_link_graph},
};

/// One authored ganger placement — its `(cell, level)` plus every E1.2 component
/// VALUE, the E3.0 attribute stats, and the read-only roster armor to seed its
/// battle-local [`WornArmor`].
///
/// A named struct (not a bare tuple) so the authored ganger shape is
/// self-describing. The component fields are the E1.2 newtypes carried **by value**
/// ([`setup_battle`] spawns an entity with each as a component) plus the E3.0 / GTW-182
/// attribute stats ([`Shooting`] / [`Toughness`] / [`Luck`], the substrate the
/// severity roll reads); `armor` is the E1.3 read-only [`SourceArmor`] record
/// ([`WornArmor::seed_from`] copies it onto the spawned entity). The grid key
/// [`at`](GangerSpawn::at) becomes the spawned ganger's [`Position`].
///
/// Not `Eq` / `Hash`: the E3.0 attribute stats ([`Shooting`] / [`Toughness`] /
/// [`Luck`]) carry `f32` magnitudes (no total order), so the authored ganger is
/// `PartialEq` only. `(cell, level)`-keyed de-duplication ([`has_stacked_gangers`])
/// hashes [`at`](GangerSpawn::at), never the whole struct.
///
/// Derives [`Deserialize`] so an authored situation `.ron` names each ganger's
/// placement + every component VALUE + roster armor (the value graph all flows
/// through the landed newtype/enum serde derives — render-free, pixel-free).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct GangerSpawn {
    /// The `(cell, level)` the ganger spawns at — its [`Position`].
    pub at:         CellLevel,
    /// The ganger's gang (faction) identity.
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's hit-points pool.
    pub hp:         Hp,
    /// The ganger's Wounds (life) pool.
    pub wounds:     Wounds,
    /// The ganger's Time-Unit budget.
    pub tu:         Tu,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
    /// The ganger's **Shooting** combat stat — the ranged-to-hit skill term the
    /// §1b concentration exponent reads (E3.0 / GTW-182).
    pub shooting:   Shooting,
    /// The ganger's **Toughness** attribute — the defender's severity-roll
    /// mitigation (E3.0 / GTW-182).
    pub toughness:  Toughness,
    /// The ganger's **Luck** attribute — directional fortune shaping the severity
    /// roll's one-sided tail (E3.0 / GTW-182).
    pub luck:       Luck,
    /// The ganger's read-only roster armor (`armor_by_part`) — copied into a
    /// battle-local [`WornArmor`] at setup, never mutated.
    pub armor:      SourceArmor,
}

/// One authored piece of cover — a wall *or* a scatter prop, the SAME schema for
/// both (`docs/combat/resolution.md` §3: "one ledger for walls *and* props").
///
/// A named struct carrying the authored cover facts [`setup_battle`] pours into
/// both the [`CoverLedger`] (its max [`CoverHp`] / [`HeightBand`] / armor) and the
/// [`OccupancyGrid`] terrain (its [`TerrainKind`]). Walls and scatter differ only
/// in which [`Situation`] list they live in ([`walls`](Situation::walls) vs
/// [`scatter`](Situation::scatter)) — the cover model treats them identically.
///
/// Derives [`Deserialize`] so an authored situation `.ron` names each piece's
/// `(cell, level)` + terrain kind + cover-HP + height band + armor stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct CoverSpawn {
    /// The `(cell, level)` this cover occupies.
    pub at:               CellLevel,
    /// The terrain kind this cover marks in the occupancy grid (wall / cover).
    pub terrain:          TerrainKind,
    /// The cover's full (max) structural HP — seeds the ledger entry's
    /// `current_hp == max_hp`.
    pub cover_hp:         CoverHp,
    /// The clearance band this cover occupies (LOW / MID / HIGH).
    pub height_band:      HeightBand,
    /// The cover's damage-reduction stat (same armor model as a ganger).
    pub armor_protection: ArmorProtection,
    /// The penetration this cover shrugs off (same armor model as a ganger).
    pub armor_hardness:   ArmorHardness,
}

impl CoverSpawn {
    /// Build an authored cover piece from its `(cell, level)`, terrain kind, max
    /// HP, height band, and armor stats.
    #[must_use]
    pub const fn new(
        at: CellLevel,
        terrain: TerrainKind,
        cover_hp: CoverHp,
        height_band: HeightBand,
        armor_protection: ArmorProtection,
        armor_hardness: ArmorHardness,
    ) -> Self {
        Self {
            at,
            terrain,
            cover_hp,
            height_band,
            armor_protection,
            armor_hardness,
        }
    }

    /// The ledger [`CoverEntry`] this authored piece seeds — `current_hp == max_hp`,
    /// not destroyed, with this piece's band and armor stats.
    #[must_use]
    pub const fn cover_entry(&self) -> CoverEntry {
        CoverEntry::seeded(
            self.cover_hp,
            self.height_band,
            self.armor_protection,
            self.armor_hardness,
        )
    }
}

/// The canonical authored **situation** — the full generated / authored
/// battlefield the battle is built from: gangers, walls, scatter, upper-floor
/// slabs, and stair/ladder vertical links (this [`crate::situation`] module is the
/// setup-on-entry source of truth).
///
/// This is the ONE situation type: it supersedes the GTW-156 placeholder (now
/// [`crate::occupancy::OccupancyInput`], the grid construction input) and GTW-160's
/// `vertical_links` extension (moved here). [`setup_battle`] reads it to build the
/// battle in the ECS world.
///
/// Derives [`Deserialize`] (GTW-205 / E10.3) so an authored battlefield ships as a
/// loose `.ron` file loaded through the [`RonAsset<T>`](gdtf_assets::RonAsset)
/// loader — render-free and pixel-free, the whole value graph routed through the
/// landed newtype/enum serde derives. `#[serde(default)]` on each list lets an
/// authored file omit a section it does not use (an empty battlefield deserializes
/// from `()`), matching the [`Default`] empty situation. The in-test fixture is a
/// test helper; the shipped `.ron` source is the real input.
///
/// Derives [`TypePath`] (render-free reflection metadata, no rendering) because the
/// [`RonAsset<Situation>`](gdtf_assets::RonAsset) the loader wraps it in requires
/// its payload to be [`TypePath`] — the same bound the theme spec satisfies.
#[derive(Debug, Clone, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct Situation {
    /// The authored gangers, each a [`GangerSpawn`] (placement + component values +
    /// roster armor).
    pub gangers:        Vec<GangerSpawn>,
    /// The authored walls (each a [`CoverSpawn`]).
    pub walls:          Vec<CoverSpawn>,
    /// The authored scatter / props (each a [`CoverSpawn`], same schema as a wall).
    pub scatter:        Vec<CoverSpawn>,
    /// The `(cell, level)`s that carry a present floor / roof slab.
    pub slabs:          Vec<CellLevel>,
    /// The authored stair / ladder vertical links (E1.10 / GTW-160), moved here
    /// from the GTW-156 placeholder. The only way a ganger changes storey
    /// (`docs/combat/combat.md`).
    pub vertical_links: Vec<VerticalLink>,
}

impl Situation {
    /// Build an empty situation (no gangers, cover, slabs, or links).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every `(cell, level)` this situation **authors** a tile at — the union of
    /// its wall, scatter, and slab cells.
    ///
    /// This is the **cell-existence source** for vertical-link validation
    /// ([`build_vertical_link_graph`]): a link endpoint that does not appear here
    /// dangles off a `(cell, level)` no authored tile occupies. Gangers are NOT
    /// included — a ganger standing somewhere does not author a tile a link can
    /// attach to (`docs/combat/combat.md`: links attach to authored geometry).
    pub fn authored_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.walls
            .iter()
            .map(|c| c.at)
            .chain(self.scatter.iter().map(|c| c.at))
            .chain(self.slabs.iter().copied())
    }
}

/// The result of [`setup_battle`] — the spawned ganger placements, so the caller
/// can map each authored ganger to its newly-spawned Bevy [`Entity`](bevy::prelude::Entity) handle.
///
/// A named newtype over the placement list (no-bare-types: the setup outcome is a
/// domain value, not a bare `Vec`). Each [`OccupantPlacement`] pairs a
/// `(cell, level)` with the SPAWNED [`Entity`](bevy::prelude::Entity) handle — **never a numeric id**
/// (GTW-10 / GTW-12). The placements are in authored-ganger order. The seeded
/// [`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] / [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)
/// are inserted as resources, queried off the world rather than returned here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSetup {
    /// The spawned `(cell, level) → Entity` occupant placements, in authored order.
    pub occupants: Vec<OccupantPlacement>,
}

impl BattleSetup {
    /// The number of gangers spawned by the setup.
    #[must_use]
    pub const fn ganger_count(&self) -> usize {
        self.occupants.len()
    }
}

/// Build the battle in the ECS world from a [`Situation`] — the E1.8 setup: the
/// setup system that builds the scene from the situation (see the
/// [`crate::situation`] module doc, the setup-on-entry source of truth).
///
/// Steps, in order:
///
/// 1. **Spawn each ganger** — for every [`GangerSpawn`], `commands.spawn(...)` the
///    full per-field component set ([`Position`] from `at`, plus [`Faction`] /
///    [`Facing`] / [`Stance`] / [`Aiming`] / [`Hp`] / [`Wounds`] / [`Tu`] /
///    [`LifeState`]), the E3.0 / GTW-182 attribute stats
///    ([`Shooting`](crate::ganger::Shooting) / [`Toughness`](crate::ganger::Toughness)
///    / [`Luck`](crate::ganger::Luck)) the severity roll reads, PLUS the battle-local
///    [`WornArmor`] seeded by value from the ganger's roster [`SourceArmor`]
///    ([`WornArmor::seed_from`]). The returned Bevy [`Entity`](bevy::prelude::Entity)
///    handle is captured into the [`OccupantPlacement`] list — NEVER a numeric id
///    (GTW-10 / GTW-12).
/// 2. **Seed the [`CoverLedger`]** — insert a [`CoverEntry`] for every wall and
///    scatter piece (the one unified ledger).
/// 3. **Seed the [`SurfaceGrid`]** — set [`SlabState::Present`] at every authored
///    slab (ground damage starts at zero by lazy default).
/// 4. **Build the [`OccupancyGrid`]** — pour an [`OccupancyInput`] of the authored
///    terrain (walls + scatter → their [`TerrainKind`]) and the SPAWNED occupant
///    entities through [`OccupancyGrid::build_from_occupancy_input`].
/// 5. **Validate + build the [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)** — via
///    [`build_vertical_link_graph`]; on success insert it, on failure return the
///    typed [`InvalidVerticalLink`] (the no-panic contract). The gangers are spawned
///    and the other three resources inserted regardless — a bad vertical link does
///    not unspawn them; the caller treats the error as a setup abort.
///
/// All four resources ([`CoverLedger`], [`SurfaceGrid`], [`OccupancyGrid`],
/// [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)) are inserted via [`Commands`]. The function is
/// render-free and headless-driven (it touches no renderer / asset server), so a
/// `MinimalPlugins` test can run it directly.
///
/// # Errors
///
/// Returns [`InvalidVerticalLink`] if any authored vertical link fails validation
/// (level out of range, dangling endpoint, or same-storey) — see
/// [`build_vertical_link_graph`].
pub fn setup_battle(
    situation: &Situation,
    commands: &mut Commands,
) -> Result<BattleSetup, InvalidVerticalLink> {
    // Validate the vertical links FIRST, so a bad authored link aborts the whole
    // setup before any entity is spawned or any resource inserted (no partial,
    // unspawnable world left behind on a validation failure).
    let vertical_graph = build_vertical_link_graph(situation)?;

    // 1. Spawn each ganger with its full component set + seeded worn armor, keeping
    //    the returned Entity handle (never a numeric id — GTW-10 / GTW-12).
    let mut occupants = Vec::with_capacity(situation.gangers.len());
    for ganger in &situation.gangers {
        let entity = commands
            .spawn((
                Position::new(ganger.at),
                ganger.faction,
                ganger.facing,
                ganger.stance,
                ganger.aiming,
                ganger.hp,
                ganger.wounds,
                ganger.tu,
                ganger.life_state,
                ganger.shooting,
                ganger.toughness,
                ganger.luck,
                WornArmor::seed_from(&ganger.armor),
            ))
            .id();
        occupants.push(OccupantPlacement::new(ganger.at, entity));
    }

    // 2. Seed the cover ledger from walls + scatter (the one unified ledger).
    let mut cover_ledger = CoverLedger::new();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        cover_ledger.insert(cover.at, cover.cover_entry());
    }
    commands.insert_resource(cover_ledger);

    // 3. Seed the surface grid: every authored slab is Present (zero ground damage
    //    by lazy default).
    let mut surface_grid = SurfaceGrid::new();
    for &slab in &situation.slabs {
        surface_grid.set_slab(slab, SlabState::Present);
    }
    commands.insert_resource(surface_grid);

    // Own the placements in the result up front, so the occupancy grid can borrow
    // them (no clone) and the same Vec is returned to the caller.
    let setup = BattleSetup { occupants };

    // 4. Build the occupancy grid: terrain from walls + scatter, occupants from the
    //    spawned entities (borrowed from the result's placement list).
    let terrain: Vec<TerrainPlacement> = situation
        .walls
        .iter()
        .chain(situation.scatter.iter())
        .map(|c| TerrainPlacement::new(c.at, c.terrain))
        .collect();
    let occupancy_input = OccupancyInput {
        terrain,
        occupants: setup.occupants.clone(),
    };
    commands.insert_resource(OccupancyGrid::build_from_occupancy_input(&occupancy_input));

    // 5. The validated vertical-link graph (validation already ran above).
    commands.insert_resource(vertical_graph);

    Ok(setup)
}

/// Whether `situation`'s authored ganger cells contain a duplicate — two gangers
/// spawned on the SAME `(cell, level)`.
///
/// A setup-time sanity helper (not an error in [`setup_battle`] — the occupancy
/// pour keeps the last write, matching the GTW-156 contract — but useful for a
/// caller / test to detect an over-stacked situation). Returns `true` if any
/// `(cell, level)` is authored for more than one ganger.
#[must_use]
pub fn has_stacked_gangers(situation: &Situation) -> bool {
    let mut seen = HashSet::new();
    situation.gangers.iter().any(|g| !seen.insert(g.at))
}

#[cfg(test)]
mod tests {
    use bevy::{
        app::App,
        ecs::system::RunSystemOnce,
        prelude::{Commands, Entity, MinimalPlugins, World},
    };

    use super::*;
    use crate::{
        armor::{ArmorFloor, ArmorIntegrity, ArmorPiece, ArmorType, BodyPart},
        cover::{Destroyed, HeightBand},
        ganger::{Direction, StanceKind},
        metric::{Cell, Level},
        vertical::{LinkKind, VerticalLinkGraph},
    };

    /// Build a `(cell, level)` key from raw coordinates.
    fn key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// An arbitrary roster armor record — distinct per-part magnitudes (NOT shipped
    /// tuning) so the seed copy is provably faithful, never asserting a magnitude.
    fn arbitrary_armor(base: i32) -> SourceArmor {
        SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(base),
            ArmorProtection::new(base + 1),
            ArmorIntegrity::new(base + 2),
            ArmorHardness::new(base + 3),
            ArmorType::DEFAULT,
        ))
    }

    /// Build an authored ganger at `at` with the given faction and otherwise
    /// arbitrary-but-DISTINCT component values, so a test can prove each field
    /// lands on the spawned entity.
    fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
        GangerSpawn {
            at,
            faction: Faction::new(faction),
            facing: Facing::new(Direction::East),
            stance: Stance::new(StanceKind::Crouching),
            aiming: Aiming::new(true),
            hp: Hp::new(40),
            wounds: Wounds::new(3),
            tu: Tu::new(60),
            life_state: LifeState::Alive,
            // The E3.0 attribute stats — distinct arbitrary magnitudes per faction so
            // a per-field readback is provable (NOT shipped tuning; per-ganger data).
            shooting: Shooting::new(f32::from(faction) + 2.0),
            toughness: Toughness::new(f32::from(faction) + 3.0),
            luck: Luck::new(f32::from(faction) + 1.0),
            armor: arbitrary_armor(i32::from(faction) + 1),
        }
    }

    /// An authored wall at `at` with arbitrary cover stats.
    fn wall_at(at: CellLevel) -> CoverSpawn {
        CoverSpawn::new(
            at,
            TerrainKind::Wall,
            CoverHp::new(120),
            HeightBand::High,
            ArmorProtection::new(8),
            ArmorHardness::new(4),
        )
    }

    /// The C8 minimal fixture: 2 gangers (distinct factions + cells), 1 wall, 1
    /// slab — the SAME fixture every C8 assertion reads from.
    fn minimal_fixture() -> (Situation, CellLevel, CellLevel, CellLevel, CellLevel) {
        let alice_at = key(5, 6, 0);
        let bob_at = key(7, 8, 0);
        let wall_cell = key(1, 2, 0);
        let slab_cell = key(3, 4, 1);

        let situation = Situation {
            gangers: vec![ganger_at(alice_at, 0), ganger_at(bob_at, 1)],
            walls: vec![wall_at(wall_cell)],
            slabs: vec![slab_cell],
            ..Situation::new()
        };
        (situation, alice_at, bob_at, wall_cell, slab_cell)
    }

    /// Run [`setup_battle`] on a fresh `MinimalPlugins` app, asserting it succeeded,
    /// and return the app (so the caller queries the resulting world) plus the
    /// [`BattleSetup`] — or assert-fail and return `None` (keeping the tests free of
    /// `unwrap`/`expect`/`panic`, all denied in tests too).
    ///
    /// Drives the real `Commands` path via `run_system_once` and flushes the
    /// deferred commands via `world.flush()`.
    fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Run setup as a one-shot system reading the fixture, capturing its result.
        let outcome = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| setup_battle(&situation, &mut commands));

        // The one-shot system itself must run (Ok), and the inner setup must succeed.
        assert!(outcome.is_ok(), "the one-shot setup system must run");
        let setup = outcome.ok().and_then(Result::ok);
        assert!(
            setup.is_some(),
            "setup_battle must succeed on a valid situation",
        );
        let setup = setup?;
        // Flush the deferred Commands (spawns + insert_resource) into the world.
        app.world_mut().flush();
        Some((app, setup))
    }

    /// C8(a) — the correct entity COUNT spawned: a 2-ganger fixture spawns exactly
    /// two ganger entities (each carrying the worn-armor component) and no more.
    #[test]
    fn setup_spawns_exactly_the_authored_ganger_count() {
        let (situation, ..) = minimal_fixture();
        let Some((mut app, setup)) = run_setup(situation) else {
            return;
        };

        assert_eq!(setup.ganger_count(), 2, "two authored gangers were spawned");

        // Exactly two entities carry the per-ganger worn armor — the spawned set.
        let world: &mut World = app.world_mut();
        let mut query = world.query::<&WornArmor>();
        assert_eq!(
            query.iter(world).count(),
            2,
            "exactly two ganger entities exist in the world",
        );
    }

    /// C8(b) — each spawned ganger carries ALL required components (E1.2 set +
    /// `WornArmor`), proven by a full-tuple query matching both entities.
    #[test]
    fn each_spawned_ganger_has_all_required_components() {
        let (situation, ..) = minimal_fixture();
        let Some((mut app, _setup)) = run_setup(situation) else {
            return;
        };

        let world: &mut World = app.world_mut();
        // A query naming EVERY required component — only an entity carrying all of
        // them matches, so a count of 2 proves both gangers have the full set
        // (E1.2 state + E3.0 attribute stats + WornArmor).
        let mut all = world.query::<(
            &Position,
            &Faction,
            &Facing,
            &Stance,
            &Aiming,
            &Hp,
            &Wounds,
            &Tu,
            &LifeState,
            &Shooting,
            &Toughness,
            &Luck,
            &WornArmor,
        )>();
        assert_eq!(
            all.iter(world).count(),
            2,
            "both gangers must carry the full E1.2 set + E3.0 attribute stats + WornArmor",
        );
    }

    /// C8(c) — each ganger's Position and Faction match the fixture, looked up by
    /// the spawned Entity handle (never a numeric id).
    #[test]
    fn spawned_position_and_faction_match_the_fixture() {
        let (situation, alice_at, bob_at, ..) = minimal_fixture();
        let Some((mut app, setup)) = run_setup(situation) else {
            return;
        };

        // The setup returns placements in authored order: alice (faction 0) then
        // bob (faction 1).
        let placements = &setup.occupants;
        assert_eq!(placements.len(), 2);

        let world: &mut World = app.world_mut();
        let mut q = world.query::<(&Position, &Faction)>();

        // Alice — placement 0.
        let alice: Entity = placements[0].occupant;
        assert_eq!(placements[0].at, alice_at);
        let alice_components = q.get(world, alice);
        assert!(
            alice_components.is_ok(),
            "alice's spawned entity must carry Position + Faction",
        );
        let Ok((alice_pos, alice_faction)) = alice_components else {
            return;
        };
        assert_eq!(
            *alice_pos,
            Position::new(alice_at),
            "alice Position matches"
        );
        assert_eq!(*alice_faction, Faction::new(0), "alice Faction matches");

        // Bob — placement 1.
        let bob: Entity = placements[1].occupant;
        assert_eq!(placements[1].at, bob_at);
        let bob_components = q.get(world, bob);
        assert!(
            bob_components.is_ok(),
            "bob's spawned entity must carry Position + Faction",
        );
        let Ok((bob_pos, bob_faction)) = bob_components else {
            return;
        };
        assert_eq!(*bob_pos, Position::new(bob_at), "bob Position matches");
        assert_eq!(*bob_faction, Faction::new(1), "bob Faction matches");

        // The two are distinct Entity handles.
        assert_ne!(alice, bob, "the two gangers are distinct entities");
    }

    /// GTW-182 AC #2 + AC #3 — `setup_battle` seeds the E3.0 attribute stats
    /// (`Shooting`/`Toughness`/`Luck`) onto each spawned ganger from the authored
    /// `GangerSpawn`, and they are queryable off the entity by its spawned `Entity`
    /// handle (the read shape the severity roll uses). Reads BOTH gangers — a shooter
    /// (faction 0) and a defender (faction 1) — proving the per-ganger seed, not a
    /// shared default. Magnitudes match the fixture (`faction + {2,3,1}`), per-ganger
    /// data, not pinned tuning.
    #[test]
    fn setup_seeds_attribute_stats_onto_each_ganger() {
        let (situation, ..) = minimal_fixture();
        let Some((mut app, setup)) = run_setup(situation) else {
            return;
        };

        let alice: Entity = setup.occupants[0].occupant;
        let bob: Entity = setup.occupants[1].occupant;
        let world: &mut World = app.world_mut();
        // The severity-roll read shape: a tuple query over the three attribute stats.
        let mut q = world.query::<(&Shooting, &Toughness, &Luck)>();

        // Alice — faction 0 → Shooting 2.0 / Toughness 3.0 / Luck 1.0 (the fixture).
        let alice_stats = q.get(world, alice);
        assert_eq!(
            alice_stats,
            Ok((&Shooting::new(2.0), &Toughness::new(3.0), &Luck::new(1.0))),
            "alice carries her authored Shooting/Toughness/Luck",
        );

        // Bob — faction 1 → Shooting 3.0 / Toughness 4.0 / Luck 2.0 (distinct seed).
        let bob_stats = q.get(world, bob);
        assert_eq!(
            bob_stats,
            Ok((&Shooting::new(3.0), &Toughness::new(4.0), &Luck::new(2.0))),
            "bob carries his authored Shooting/Toughness/Luck",
        );
    }

    /// C8(d) — the worn armor on a spawned ganger equals the fixture's roster armor,
    /// field-by-field across all six parts (E1.3 seed-from on the real spawn path).
    #[test]
    fn spawned_worn_armor_matches_the_fixture_roster() {
        let (situation, ..) = minimal_fixture();
        // The fixture's alice roster armor (faction 0 → base 1).
        let expected = arbitrary_armor(1);
        let Some((mut app, setup)) = run_setup(situation) else {
            return;
        };

        let alice: Entity = setup.occupants[0].occupant;
        let world: &mut World = app.world_mut();
        let mut q = world.query::<&WornArmor>();
        let worn_result = q.get(world, alice);
        assert!(
            worn_result.is_ok(),
            "alice must carry a WornArmor component",
        );
        let Ok(worn) = worn_result else {
            return;
        };
        for part in BodyPart::ALL {
            assert_eq!(
                worn.at(part),
                expected.at(part),
                "worn armor at {part:?} must equal the seeded roster piece",
            );
        }
    }

    /// C8(d) — the `CoverLedger` is seeded from the SAME fixture: the wall cell
    /// holds a full-HP, not-destroyed entry with the authored band + armor.
    #[test]
    fn cover_ledger_seeded_from_fixture_wall() {
        let (situation, _alice, _bob, wall_cell, _slab) = minimal_fixture();
        let expected = wall_at(wall_cell).cover_entry();
        let Some((app, _setup)) = run_setup(situation) else {
            return;
        };

        let ledger = app.world().get_resource::<CoverLedger>();
        assert!(ledger.is_some(), "setup must insert a CoverLedger resource");
        let Some(ledger) = ledger else {
            return;
        };
        // The wall entry is present (registered, not just lazily seedable) and equal
        // to the authored prototype.
        assert_eq!(
            ledger.peek(&wall_cell).copied(),
            Some(expected),
            "the ledger must hold the authored wall entry at the wall cell",
        );
        // It is full-HP and not destroyed (seeded shape).
        assert_eq!(
            ledger.peek(&wall_cell).map(|e| e.destroyed),
            Some(Destroyed::new(false)),
            "a freshly seeded wall is not destroyed",
        );
        // current_hp seeds to the authored max (the C4 seed invariant
        // current_hp == max_hp) — the GTW-154 precedent, not a magnitude pin.
        assert_eq!(
            ledger.peek(&wall_cell).map(|e| e.current_hp),
            ledger.peek(&wall_cell).map(|e| e.max_hp),
            "current_hp seeds to the authored max",
        );
    }

    /// C8(d) — the `SurfaceGrid` is seeded from the SAME fixture: the slab cell
    /// reads Present, an un-authored cell reads Absent.
    #[test]
    fn surface_grid_seeded_from_fixture_slab() {
        let (situation, .., slab_cell) = minimal_fixture();
        let Some((app, _setup)) = run_setup(situation) else {
            return;
        };

        let surface = app.world().get_resource::<SurfaceGrid>();
        assert!(
            surface.is_some(),
            "setup must insert a SurfaceGrid resource"
        );
        let Some(surface) = surface else {
            return;
        };
        assert_eq!(
            surface.slab_state(&slab_cell),
            SlabState::Present,
            "the authored slab must read Present",
        );
        assert_eq!(
            surface.slab_state(&key(0, 0, 1)),
            SlabState::Absent,
            "an un-authored cell must read Absent",
        );
    }

    /// C8(d) — the `OccupancyGrid` is seeded from the SAME fixture: the wall cell
    /// carries Wall terrain and blocks; each ganger cell carries its SPAWNED Entity
    /// handle as the occupant (never a numeric id).
    #[test]
    fn occupancy_grid_seeded_from_fixture() {
        let (situation, alice_at, bob_at, wall_cell, _slab) = minimal_fixture();
        let Some((app, setup)) = run_setup(situation) else {
            return;
        };

        let grid = app.world().get_resource::<OccupancyGrid>();
        assert!(
            grid.is_some(),
            "setup must insert an OccupancyGrid resource"
        );
        let Some(grid) = grid else {
            return;
        };

        // Terrain slot from the wall.
        assert_eq!(
            grid.terrain(&wall_cell),
            TerrainKind::Wall,
            "the wall cell must carry Wall terrain",
        );
        assert!(grid.is_blocked(&wall_cell), "a standing wall blocks");

        // Occupant slots hold the SPAWNED Entity handles (matching the returned
        // placements), never a numeric id.
        let alice = setup.occupants[0].occupant;
        let bob = setup.occupants[1].occupant;
        assert_eq!(
            grid.occupant(&alice_at),
            Some(alice),
            "alice's cell holds alice's spawned Entity handle",
        );
        assert_eq!(
            grid.occupant(&bob_at),
            Some(bob),
            "bob's cell holds bob's spawned Entity handle",
        );
    }

    /// The `VerticalLinkGraph` is built + inserted from a valid authored link, and
    /// is queryable from the world.
    #[test]
    fn setup_inserts_vertical_link_graph() {
        let lower = key(1, 2, 0);
        let upper = key(1, 2, 1);
        let situation = Situation {
            // Author both endpoint cells as slabs so the link does not dangle.
            slabs: vec![lower, upper],
            vertical_links: vec![VerticalLink::new(lower, upper, LinkKind::stair())],
            ..Situation::new()
        };

        let Some((app, _setup)) = run_setup(situation) else {
            return;
        };

        let graph = app.world().get_resource::<VerticalLinkGraph>();
        assert!(
            graph.is_some(),
            "setup must insert a VerticalLinkGraph resource",
        );
        let Some(graph) = graph else {
            return;
        };
        assert_eq!(graph.len(), 1, "the one valid authored link is indexed");
        assert_eq!(
            graph.links_from(&lower).count(),
            1,
            "the link departs the lower endpoint",
        );
    }

    /// A bad authored vertical link (dangling endpoint) makes `setup_battle` return
    /// `Err(DanglingCell)` and spawn NOTHING — validation runs first, aborting the
    /// setup before any entity or resource is created (the no-panic contract).
    #[test]
    fn setup_aborts_on_invalid_vertical_link() {
        let present = key(4, 4, 0);
        let missing = key(4, 4, 1); // never authored
        let link = VerticalLink::new(present, missing, LinkKind::stair());
        let situation = Situation {
            gangers: vec![ganger_at(key(0, 0, 0), 0)],
            slabs: vec![present], // only `present` authored — `missing` dangles
            vertical_links: vec![link],
            ..Situation::new()
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let result = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| setup_battle(&situation, &mut commands));

        // The one-shot system ran; the inner setup returned the typed error.
        assert!(result.is_ok(), "the one-shot system must run");
        let Ok(setup_result) = result else {
            return;
        };
        assert_eq!(
            setup_result.err(),
            Some(InvalidVerticalLink::DanglingCell { link }),
            "an invalid vertical link must abort setup with the typed error",
        );

        // Nothing was spawned (validation aborted before the spawn loop).
        app.world_mut().flush();
        let world = app.world_mut();
        let mut q = world.query::<&WornArmor>();
        assert_eq!(
            q.iter(world).count(),
            0,
            "a validation abort must spawn no gangers",
        );
        assert!(
            world.get_resource::<CoverLedger>().is_none(),
            "a validation abort must insert no resources",
        );
    }

    /// `authored_cells` is the union of wall, scatter, and slab cells — NOT ganger
    /// cells (a ganger does not author a tile a link can attach to).
    #[test]
    fn authored_cells_unions_walls_scatter_slabs_only() {
        let wall = key(1, 1, 0);
        let prop = key(2, 2, 0);
        let slab = key(3, 3, 1);
        let ganger = key(9, 9, 0);
        let situation = Situation {
            gangers: vec![ganger_at(ganger, 0)],
            walls: vec![wall_at(wall)],
            scatter: vec![CoverSpawn::new(
                prop,
                TerrainKind::Cover,
                CoverHp::new(20),
                HeightBand::Low,
                ArmorProtection::new(1),
                ArmorHardness::new(0),
            )],
            slabs: vec![slab],
            ..Situation::new()
        };

        let cells: HashSet<CellLevel> = situation.authored_cells().collect();
        assert!(cells.contains(&wall), "wall cell is authored");
        assert!(cells.contains(&prop), "scatter cell is authored");
        assert!(cells.contains(&slab), "slab cell is authored");
        assert!(
            !cells.contains(&ganger),
            "a ganger cell does NOT author a tile",
        );
        assert_eq!(cells.len(), 3, "exactly the three terrain/slab cells");
    }

    /// `has_stacked_gangers` detects two gangers on one `(cell, level)` and is false
    /// for distinct cells.
    #[test]
    fn stacked_ganger_detection() {
        let at = key(5, 5, 0);
        let stacked = Situation {
            gangers: vec![ganger_at(at, 0), ganger_at(at, 1)],
            ..Situation::new()
        };
        assert!(
            has_stacked_gangers(&stacked),
            "two gangers on one cell stack"
        );

        let (clean, ..) = minimal_fixture();
        assert!(
            !has_stacked_gangers(&clean),
            "distinct ganger cells do not stack",
        );
    }

    // --- GTW-205 / E10.3: the Situation value graph is serde-deserializable, and
    // the SHIPPED authored file parses + drives the real setup path. The tests are
    // value-AGNOSTIC on tunables — they assert structural relations (counts,
    // distinct factions, Ok), never a pinned hp/tu/armor magnitude (those are
    // authored data, not pinned by the test).

    /// The shipped authored situation file, read at compile time via the same
    /// `include_str!` pattern `tuning.rs` uses for the shipped `tuning.ron` — the
    /// REAL on-disk path (`assets/situations/skirmish.ron`), so a regression in the
    /// authored file turns these tests red.
    const SHIPPED_SITUATION_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/situations/skirmish.ron"
    ));

    /// GTW-205 AC1 — an inline RON `Situation` containing ≥1 ganger, ≥1 wall, ≥1
    /// scatter piece, ≥1 slab, and ≥1 vertical link deserializes to `Ok`, and the
    /// resulting value's list lengths equal the authored counts. Count-equality, not
    /// a tunable magnitude — proving the whole spawn-struct value graph is
    /// serde-deserializable through the landed newtype/enum derives.
    #[test]
    fn situation_deserializes_from_inline_ron_with_each_section() {
        // A minimal-but-complete authored situation: one of every section. The
        // single vertical link's endpoints are authored slabs on different storeys
        // (non-dangling, cross-storey), so the value is setup-able.
        let authored = "(
            gangers: [(
                at: (cell: (x: 0, y: 0), level: 0),
                faction: 0, facing: North, stance: Standing, aiming: false,
                hp: 10, wounds: 2, tu: 30, life_state: Alive,
                shooting: 1.0, toughness: 1.0, luck: 0.0,
                armor: (
                    head:      (floor: 0, protection: 1, integrity: 5, hardness: 0, armor_type: Plated),
                    torso:     (floor: 0, protection: 1, integrity: 5, hardness: 0, armor_type: Plated),
                    left_arm:  (floor: 0, protection: 1, integrity: 5, hardness: 0, armor_type: Plated),
                    right_arm: (floor: 0, protection: 1, integrity: 5, hardness: 0, armor_type: Plated),
                    left_leg:  (floor: 0, protection: 1, integrity: 5, hardness: 0, armor_type: Plated),
                    right_leg: (floor: 0, protection: 1, integrity: 5, hardness: 0, armor_type: Plated),
                ),
            )],
            walls: [(
                at: (cell: (x: 1, y: 1), level: 0), terrain: Wall, cover_hp: 50,
                height_band: High, armor_protection: 4, armor_hardness: 2,
            )],
            scatter: [(
                at: (cell: (x: 2, y: 2), level: 0), terrain: Cover, cover_hp: 10,
                height_band: Low, armor_protection: 1, armor_hardness: 0,
            )],
            slabs: [
                (cell: (x: 3, y: 3), level: 0),
                (cell: (x: 3, y: 3), level: 1),
            ],
            vertical_links: [(
                from: (cell: (x: 3, y: 3), level: 0),
                to: (cell: (x: 3, y: 3), level: 1),
                kind: Stair(one_way: false),
            )],
        )";

        let parsed = ron::de::from_str::<Situation>(authored);
        assert!(
            parsed.is_ok(),
            "inline Situation RON must parse: {parsed:?}"
        );
        let Ok(situation) = parsed else {
            return;
        };
        // Count-equality with the authored sections — never a magnitude.
        assert_eq!(situation.gangers.len(), 1, "one authored ganger");
        assert_eq!(situation.walls.len(), 1, "one authored wall");
        assert_eq!(situation.scatter.len(), 1, "one authored scatter piece");
        assert_eq!(situation.slabs.len(), 2, "two authored slabs");
        assert_eq!(
            situation.vertical_links.len(),
            1,
            "one authored vertical link",
        );
    }

    /// Parse the shipped `assets/situations/skirmish.ron` into a `Situation`, once,
    /// for the AC3/AC4 tests — or assert-fail and return `None` (keeping the tests
    /// free of `unwrap`/`expect`/`panic`, all denied in tests too).
    fn shipped_situation() -> Option<Situation> {
        let parsed = ron::de::from_str::<Situation>(SHIPPED_SITUATION_RON);
        assert!(
            parsed.is_ok(),
            "shipped assets/situations/skirmish.ron must deserialize into Situation: {parsed:?}",
        );
        parsed.ok()
    }

    /// GTW-205 AC3 — the shipped situation file parses back into a `Situation`
    /// (value-agnostic round-trip). Asserts only STRUCTURAL relations: gangers
    /// non-empty, ≥2 distinct factions present, ≥1 wall, ≥1 scatter, ≥1 slab, ≥1
    /// vertical link — NEVER a specific hp/tu/armor magnitude (authored data, not
    /// pinned by the test).
    #[test]
    fn shipped_situation_ron_deserializes_with_required_structure() {
        let Some(situation) = shipped_situation() else {
            return;
        };

        assert!(
            !situation.gangers.is_empty(),
            "the shipped file must author at least one ganger",
        );
        // ≥2 distinct factions present (two gangs face off).
        let distinct_factions: HashSet<_> = situation.gangers.iter().map(|g| g.faction).collect();
        assert!(
            distinct_factions.len() >= 2,
            "the shipped file must author at least two distinct factions, found {}",
            distinct_factions.len(),
        );
        assert!(
            !situation.walls.is_empty(),
            "the shipped file must author at least one wall",
        );
        assert!(
            !situation.scatter.is_empty(),
            "the shipped file must author at least one scatter piece",
        );
        assert!(
            !situation.slabs.is_empty(),
            "the shipped file must author at least one slab",
        );
        assert!(
            !situation.vertical_links.is_empty(),
            "the shipped file must author at least one vertical link",
        );
    }

    /// GTW-205 AC4 — the shipped file's vertical links validate (non-dangling /
    /// cross-storey), proving it is a setup-able situation. Deserialize the shipped
    /// file, run `setup_battle` on a `MinimalPlugins` app via the existing
    /// `run_setup` harness, and assert it returns `Ok(BattleSetup)` with
    /// `ganger_count()` equal to the authored ganger count AND exactly that many
    /// `WornArmor`-carrying entities in the world. Count-equality + Ok — proving the
    /// links are non-dangling/cross-storey and the file drives the real setup path.
    #[test]
    fn shipped_situation_ron_drives_the_real_setup_path() {
        let Some(situation) = shipped_situation() else {
            return;
        };
        let authored_ganger_count = situation.gangers.len();

        let Some((mut app, setup)) = run_setup(situation) else {
            return;
        };

        assert_eq!(
            setup.ganger_count(),
            authored_ganger_count,
            "setup must spawn exactly the authored ganger count from the shipped file",
        );
        // Exactly that many entities carry the seeded worn armor — proving the file
        // poured through the real spawn path (and that the vertical links validated,
        // since setup aborts before spawning on a bad link).
        let world: &mut World = app.world_mut();
        let mut query = world.query::<&WornArmor>();
        assert_eq!(
            query.iter(world).count(),
            authored_ganger_count,
            "the world must hold exactly the authored ganger count of WornArmor entities",
        );
    }

    /// GTW-205 AC5 — every value-bearing field in the authored `.ron` carries a
    /// per-line explanatory comment (the tuning `.ron` convention). Reads the shipped
    /// file text and asserts that every line carrying an authored LEAF value (a
    /// scalar/variant field or a list element) also carries a `//` annotation. This
    /// guards the canon-comment convention without pinning any value.
    #[test]
    fn shipped_situation_ron_is_per_line_commented() {
        for raw in SHIPPED_SITUATION_RON.lines() {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            // A full-line comment (header / section banner) is fine as-is.
            if trimmed.starts_with("//") {
                continue;
            }
            // Split off any trailing comment; the code portion is what precedes `//`.
            let code = trimmed.split("//").next().unwrap_or("").trim();
            if code.is_empty() {
                continue;
            }
            // A line that only OPENS or CLOSES a block (its code ends with a bare
            // bracket, e.g. `gangers: [`, `armor: (`, `(`, `),`, `],`) is structural:
            // the section-comment banner above it documents the block, so it needs no
            // per-line annotation. Every other code line carries an authored LEAF
            // value and MUST be annotated.
            let opens_block = code.ends_with('(') || code.ends_with('[') || code.ends_with('{');
            let closes_block = code
                .chars()
                .all(|c| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ','));
            if opens_block || closes_block {
                continue;
            }
            // A value-bearing leaf line: it MUST carry a `//` annotation somewhere.
            assert!(
                trimmed.contains("//"),
                "every value-bearing line must carry a `//` comment; bare line: {trimmed:?}",
            );
        }
    }
}
