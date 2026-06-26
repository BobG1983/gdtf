//! The authored value graph: [`GangerSpawn`], [`CoverSpawn`], [`SlabSpawn`],
//! [`FloorSpawn`], and the canonical [`Situation`] — the serde-deserializable
//! battlefield the setup is built from.

use bevy::reflect::TypePath;
use serde::Deserialize;

use crate::{
    ganger::{
        Aim, Aiming, Cool, Facing, Faction, GangerName, Grit, LifeState, Luck, Reflexes, Speed,
        Stance, Strength, Toughness,
    },
    metric::CellLevel,
    terrain::piece::TerrainName,
    vertical::VerticalLink,
    weapon::WeaponName,
};

/// One authored ganger placement — its `(cell, level)` plus its identity / posture
/// fields, the EIGHT direct attributes (GTW-384, the raw authored potential), and the
/// armor KEY whose resolved [`ArmorSpec`](crate::armor::ArmorSpec) spawns the ganger's
/// battle-local armor-piece entities (related via [`Wears`](crate::armor::Wears)).
///
/// A named struct (not a bare tuple) so the authored ganger shape is self-describing.
/// Since GTW-384 the situation authors the EIGHT DIRECT ATTRIBUTES
/// ([`Speed`] / [`Aim`] / [`Strength`] / [`Toughness`] / [`Reflexes`] / [`Cool`] /
/// [`Grit`] / [`Luck`]) — NOT the computed stats. The flat computed-stat literals it
/// used to carry (`hp` / `hp_max` / `wounds` / `wounds_max` / `tu` / `tu_max` /
/// `shooting`) are GONE: [`setup_battle`](crate::situation::setup_battle) DERIVES them
/// from the attributes × the
/// [`GangerStatTuning`](crate::tuning::GangerStatTuning) weights
/// (`docs/combat/stats.md` §"Computed combat stats", the two-layer model — fully
/// derived, single source of truth). `armor` is the armor KEY ([`ArmorName`](crate::armor::ArmorName))
/// resolved at setup against the [`ArmorRegistry`](crate::armor::ArmorRegistry) into the
/// [`ArmorSpec`](crate::armor::ArmorSpec) that the spawned ganger's battle-local
/// armor-piece entities ([`Wears`](crate::armor::Wears)) are seeded from (GTW-269 /
/// GTW-323 — mirroring the [`weapon`](GangerSpawn::weapon) key, whose resolved bundle
/// spawns the related weapon entity). The grid key [`at`](GangerSpawn::at) becomes the
/// spawned ganger's [`Position`](crate::ganger::Position).
///
/// Not `Eq` / `Hash`: the attributes carry `f32` magnitudes (no total order), so the
/// authored ganger is `PartialEq` only. `(cell, level)`-keyed de-duplication
/// ([`has_stacked_gangers`](crate::situation::has_stacked_gangers)) hashes
/// [`at`](GangerSpawn::at), never the whole struct.
///
/// Not `Copy` (GTW-257 / GTW-285 / GTW-269): the [`weapon`](GangerSpawn::weapon) key
/// is a [`WeaponName`] over a [`String`], the [`armor`](GangerSpawn::armor) key is an
/// [`ArmorName`](crate::armor::ArmorName) over a [`String`], and the
/// [`name`](GangerSpawn::name) is a [`GangerName`] over a [`String`] (all owned, not
/// `Copy`), so the authored ganger is `Clone` only. The
/// [`setup_battle`](crate::situation::setup_battle) spawn loop borrows each ganger, so
/// dropping `Copy` costs nothing on the real path.
///
/// Derives [`Deserialize`] so an authored situation `.ron` names each ganger's
/// placement + its eight attributes + roster armor + its [`weapon`](GangerSpawn::weapon)
/// key (the value graph all flows through the landed newtype/enum serde derives —
/// render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GangerSpawn {
    /// The `(cell, level)` the ganger spawns at — its [`Position`](crate::ganger::Position).
    pub at:         CellLevel,
    /// The ganger's **name** — its human-facing display identity (GTW-285).
    /// [`setup_battle`](crate::situation::setup_battle) spawns it as a [`GangerName`]
    /// component beside the rest of the per-field set; the status panel's identity line
    /// renders it (replacing the placeholder cell location). Authored per ganger in the
    /// situation `.ron` as a bare string ([`GangerName`] is `#[serde(transparent)]`).
    pub name:       GangerName,
    /// The ganger's gang (faction) identity.
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
    /// The ganger's **Speed** direct attribute — quickness (GTW-384). Drives the
    /// derived [`Tu`](crate::ganger::Tu) budget + Fight/Reactions terms. Authored as a
    /// bare scalar ([`Speed`] is `#[serde(transparent)]`).
    pub speed:      Speed,
    /// The ganger's **Aim** direct attribute — innate marksmanship (GTW-384). The
    /// dominant derived [`Shooting`](crate::ganger::Shooting) term. Authored as a bare
    /// scalar ([`Aim`] is `#[serde(transparent)]`).
    pub aim:        Aim,
    /// The ganger's **Strength** direct attribute — physical power (GTW-384). A derived
    /// Fight term. Authored as a bare scalar ([`Strength`] is `#[serde(transparent)]`).
    pub strength:   Strength,
    /// The ganger's **Toughness** direct attribute — damage resistance. REUSED (the §6
    /// severity roll already reads it); ALSO a term in the derived
    /// [`Hp`](crate::ganger::Hp) pool (GTW-384). Authored as a bare scalar.
    pub toughness:  Toughness,
    /// The ganger's **Reflexes** direct attribute — reaction speed (GTW-384). A derived
    /// Shooting + Reactions term. Authored as a bare scalar ([`Reflexes`] is
    /// `#[serde(transparent)]`).
    pub reflexes:   Reflexes,
    /// The ganger's **Cool** direct attribute — nerves under fire (GTW-384). The broad
    /// Shooting/Fight/Reactions/HP/Morale contributor. Authored as a bare scalar
    /// ([`Cool`] is `#[serde(transparent)]`).
    pub cool:       Cool,
    /// The ganger's **Grit** direct attribute — resilience (GTW-384). The dominant
    /// derived [`Hp`](crate::ganger::Hp) + Morale term. Authored as a bare scalar
    /// ([`Grit`] is `#[serde(transparent)]`).
    pub grit:       Grit,
    /// The ganger's **Luck** direct attribute — directional fortune. REUSED (the §6
    /// severity roll reads it; feeds the severity roll ONLY, never the computed stats —
    /// `docs/combat/stats.md`). Authored as a bare scalar.
    pub luck:       Luck,
    /// The ganger's **armor KEY** — the filename stem of an `assets/content/armor/*.armor.ron`
    /// (e.g. `"flak_vest"`), resolved against the
    /// [`ArmorRegistry`](crate::armor::ArmorRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the
    /// [`ArmorSpec`](crate::armor::ArmorSpec) that the spawned ganger's battle-local
    /// armor-piece entities ([`Wears`](crate::armor::Wears)) are seeded from by value —
    /// never mutated on the roster (GTW-269 / GTW-323, mirroring the
    /// [`weapon`](GangerSpawn::weapon) key). A key absent from the registry is a handled
    /// [`BattleSetupError::ArmorNotFound`](crate::situation::BattleSetupError::ArmorNotFound)
    /// error (no panic).
    pub armor:      crate::armor::ArmorName,
    /// The ganger's **weapon KEY** — the filename stem of an `assets/content/weapons/*.ron`
    /// (e.g. `"stub_pistol"`), resolved against the
    /// [`WeaponRegistry`](crate::weapon::WeaponRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the
    /// [`WeaponBundle`](crate::weapon::WeaponBundle) spawned onto the related weapon
    /// entity ([`Wields`](crate::weapon::Wields), GTW-257 / GTW-323). REQUIRED — every
    /// authored ganger is armed; an
    /// unarmed `Option<WeaponName>` case is a deliberate FUTURE option (the
    /// [[weapons-armor-data-driven]] model arms every ganger for now). A key absent
    /// from the registry is a handled
    /// [`BattleSetupError::WeaponNotFound`](crate::situation::BattleSetupError::WeaponNotFound)
    /// error (no panic).
    pub weapon:     WeaponName,
}

/// One authored piece of cover — a wall *or* a scatter prop.
///
/// GTW-396 migration: the old inline stat fields
/// (`cover_hp` / `height_band` / `armor_protection` / `armor_hardness` / `terrain`)
/// are REPLACED by a single terrain piece KEY ([`piece`](CoverSpawn::piece)) resolved
/// against the [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) at setup —
/// exactly mirroring the weapon/armor key model. The kind
/// ([`TerrainKind`](crate::occupancy::TerrainKind) / `TerrainPieceKind`) and structural
/// stats ([`CoverEntry`](crate::cover::CoverEntry)) are DERIVED from the resolved spec
/// variant at `setup_battle` time (a `Wall` spec → `TerrainKind::Wall`; a `Cover` or
/// `Scatter` spec → `TerrainKind::Cover`).
///
/// A named struct (`at` + `piece`) so the authored shape is self-describing.
/// Derives [`Deserialize`] so an authored situation `.ron` can name each piece's
/// `(cell, level)` + its terrain piece key (round-trips through the landed newtype
/// serde derives — render-free, pixel-free).
///
/// Walls and scatter differ only in which [`Situation`] list they live in
/// ([`walls`](Situation::walls) vs [`scatter`](Situation::scatter)) — the cover model
/// treats them identically.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct CoverSpawn {
    /// The `(cell, level)` this cover piece occupies.
    pub at:    CellLevel,
    /// The terrain piece KEY — the filename stem (without the `.terrain.ron` infix) of
    /// an `assets/content/terrain/*.terrain.ron` (e.g. `"heavy_bulkhead"`), resolved against the
    /// [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle). A key absent from the registry
    /// is a handled
    /// [`BattleSetupError::TerrainNotFound`](crate::situation::BattleSetupError::TerrainNotFound)
    /// error (no panic).
    pub piece: TerrainName,
}

impl CoverSpawn {
    /// Build an authored cover piece from its `(cell, level)` and terrain piece key.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainName) -> Self {
        Self { at, piece }
    }
}

/// One authored floor / roof slab — its `(cell, level)` and a terrain piece KEY.
///
/// GTW-396 migration: the old `slabs: Vec<CellLevel>` (bare cell list with no
/// per-slab authored HP) is replaced by `slabs: Vec<SlabSpawn>`, where each slab now
/// carries a terrain piece KEY resolved against the
/// [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) at setup. This brings
/// per-slab authored HP/armor out of the uniform `slab_defaults` tuning leaf and into
/// per-piece `assets/content/terrain/*.terrain.ron` data (e.g. `"deck_slab"`).
///
/// Derives [`Deserialize`] so an authored situation `.ron` writes each slab as
/// `(at: (cell: …, level: …), piece: "…")` — the same shape as [`CoverSpawn`], but
/// into the `slabs` list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct SlabSpawn {
    /// The `(cell, level)` this slab occupies (same meaning as the old bare entry).
    pub at:    CellLevel,
    /// The terrain piece KEY — the filename stem of an `assets/content/terrain/*.terrain.ron`
    /// (e.g. `"deck_slab"`), resolved against the
    /// [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the [`SlabPieceSpec`](crate::terrain::piece::SlabPieceSpec)
    /// that seeds this slab's structural HP/armor in both the
    /// [`SlabLedger`](crate::slab::SlabLedger) and the spawned terrain entity
    /// (GTW-395/396 — per-slab authored HP, not uniform tuning). A key absent from
    /// the registry is a handled
    /// [`BattleSetupError::TerrainNotFound`](crate::situation::BattleSetupError::TerrainNotFound)
    /// error (no panic).
    pub piece: TerrainName,
}

impl SlabSpawn {
    /// Build a slab spawn from its cell + terrain piece key.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainName) -> Self {
        Self { at, piece }
    }
}

/// One authored per-cell floor override — a `(cell, level)` paired with a terrain
/// piece KEY that gives it a different move cost than the situation's
/// [`default_floor`](Situation::default_floor).
///
/// GTW-396 Decision B: the [`Situation`] carries a sparse `floors` list for cells
/// whose floor cost deviates from the default. An empty `floors` list (the common
/// case — a uniform floor) is the cheapest authored state and the `#[serde(default)]`
/// for this field.
///
/// Derives [`Deserialize`] so an authored situation `.ron` writes each override as
/// `(at: (cell: …, level: …), piece: "…")`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct FloorSpawn {
    /// The `(cell, level)` with a non-default floor cost.
    pub at:    CellLevel,
    /// The terrain piece KEY (must resolve to a
    /// [`TerrainKindSpec::Floor`](crate::terrain::piece::TerrainKindSpec::Floor) variant)
    /// giving this cell its move cost.
    pub piece: TerrainName,
}

impl FloorSpawn {
    /// Build a floor spawn from its cell + terrain piece key.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainName) -> Self {
        Self { at, piece }
    }
}

/// The canonical authored **situation** — the full generated / authored
/// battlefield the battle is built from: gangers, walls, scatter, upper-floor
/// slabs, stair/ladder vertical links, and the floor cost surface (this
/// [`crate::situation`] module is the setup-on-entry source of truth).
///
/// This is the ONE situation type: it supersedes the GTW-156 placeholder (now
/// [`crate::occupancy::OccupancyInput`], the grid construction input) and GTW-160's
/// `vertical_links` extension (moved here). [`setup_battle`](crate::situation::setup_battle)
/// reads it to build the battle in the ECS world.
///
/// GTW-396 migration (schema change): `walls`/`scatter` entries now carry a terrain
/// piece KEY ([`CoverSpawn::piece`]) instead of inline stats; `slabs` entries are now
/// [`SlabSpawn`] structs carrying a piece KEY alongside their `at` cell (replacing the
/// old `Vec<CellLevel>` bare list); `default_floor` names the walkable floor piece for
/// all open cells; `floors` gives sparse per-cell floor overrides.
///
/// Derives [`Deserialize`] (GTW-205 / E10.3) so an authored battlefield ships as a
/// loose `.ron` file loaded through the `RonAsset<T>` loader — render-free and
/// pixel-free, the whole value graph routed through the landed newtype/enum serde
/// derives. `#[serde(default)]` on each list lets an authored file omit a section it
/// does not use (an empty battlefield deserializes from `()`), matching the [`Default`]
/// empty situation. The in-test fixture is a test helper; the shipped `.ron` source is
/// the real input.
///
/// Derives [`TypePath`] (render-free reflection metadata, no rendering) because the
/// `RonAsset<Situation>` the loader wraps it in requires
/// its payload to be [`TypePath`] — the same bound the theme spec satisfies.
#[derive(Debug, Clone, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct Situation {
    /// The authored gangers, each a [`GangerSpawn`] (placement + component values +
    /// roster armor).
    pub gangers:        Vec<GangerSpawn>,
    /// The authored walls (each a [`CoverSpawn`] — `at` + piece KEY).
    pub walls:          Vec<CoverSpawn>,
    /// The authored scatter / props (each a [`CoverSpawn`], same schema as a wall).
    pub scatter:        Vec<CoverSpawn>,
    /// The authored floor / roof slabs (each a [`SlabSpawn`] — `at` + piece KEY).
    /// GTW-396: was `Vec<CellLevel>`; now `Vec<SlabSpawn>` so each slab carries its
    /// terrain piece KEY for per-slab HP/armor resolution against the
    /// [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry).
    pub slabs:          Vec<SlabSpawn>,
    /// The authored stair / ladder vertical links (E1.10 / GTW-160), moved here
    /// from the GTW-156 placeholder. The only way a ganger changes storey
    /// (`docs/combat/combat.md`).
    pub vertical_links: Vec<VerticalLink>,
    /// The gang the human player controls — every other [`Faction`] is the enemy.
    /// Seeds the [`PlayerFaction`](crate::PlayerFaction) battle-lifetime resource the
    /// later control-gating + victory-census slices read. The struct-level
    /// `#[serde(default)]` supplies [`Faction::default`] = `Faction(0)` for any
    /// authored file that omits the field, so every existing situation `.ron` stays
    /// valid (gang `0` is the player by convention, matching `skirmish.ron`); an
    /// authored `player_faction: 1` parses as the bare gang index
    /// ([`Faction`] is `#[serde(transparent)]`).
    pub player_faction: Faction,
    /// The default floor terrain piece KEY — the filename stem (without the
    /// `.terrain.ron` infix) of an `assets/content/terrain/*.terrain.ron` that is a
    /// [`TerrainKindSpec::Floor`](crate::terrain::piece::TerrainKindSpec::Floor)
    /// variant. Applied to every walkable open cell not overridden by [`floors`](Situation::floors).
    ///
    /// `#[serde(default)]` supplies the empty string sentinel (`TerrainName::default()`
    /// — [`TerrainName::is_empty`]) for any authored file that omits the field, so
    /// every EXISTING situation `.ron` remains parse-valid.  An empty default skips
    /// `FloorCostGrid` construction from the registry (the sim falls back to seeding it
    /// from `CombatTuning::move_costs.open`, preserving the pre-GTW-396 behavior for
    /// test fixtures and situations that haven't migrated yet).
    pub default_floor:  TerrainName,
    /// Sparse per-cell floor piece overrides — cells whose floor cost differs from
    /// [`default_floor`](Situation::default_floor).
    /// `#[serde(default)]` gives an empty list (the common case: uniform floor).
    pub floors:         Vec<FloorSpawn>,
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
    /// ([`build_vertical_link_graph`](crate::vertical::build_vertical_link_graph)):
    /// a link endpoint that does not appear here
    /// dangles off a `(cell, level)` no authored tile occupies. Gangers are NOT
    /// included — a ganger standing somewhere does not author a tile a link can
    /// attach to (`docs/combat/combat.md`: links attach to authored geometry).
    pub fn authored_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.walls
            .iter()
            .map(|c| c.at)
            .chain(self.scatter.iter().map(|c| c.at))
            .chain(self.slabs.iter().map(|s| s.at))
    }
}
