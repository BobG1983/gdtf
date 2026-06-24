//! The authored value graph: [`GangerSpawn`], [`CoverSpawn`], and the canonical
//! [`Situation`] — the serde-deserializable battlefield the setup is built from.

use bevy::reflect::TypePath;
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorName, ArmorProtection},
    cover::{CoverEntry, CoverHp, HeightBand},
    ganger::{
        Aim, Aiming, Cool, Facing, Faction, GangerName, Grit, LifeState, Luck, Reflexes, Speed,
        Stance, Strength, Toughness,
    },
    metric::CellLevel,
    occupancy::TerrainKind,
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
/// derived, single source of truth). `armor` is the armor KEY ([`ArmorName`]) resolved
/// at setup against the [`ArmorRegistry`](crate::armor::ArmorRegistry) into the
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
/// [`ArmorName`] over a [`String`], and the [`name`](GangerSpawn::name) is a
/// [`GangerName`] over a [`String`] (all owned, not `Copy`), so the authored ganger
/// is `Clone` only. The [`setup_battle`](crate::situation::setup_battle) spawn loop
/// borrows each ganger, so dropping `Copy` costs nothing on the real path.
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
    /// The ganger's **armor KEY** — the filename stem of an `assets/armor/*.armor.ron`
    /// (e.g. `"flak_vest"`), resolved against the
    /// [`ArmorRegistry`](crate::armor::ArmorRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the
    /// [`ArmorSpec`](crate::armor::ArmorSpec) that the spawned ganger's battle-local
    /// armor-piece entities ([`Wears`](crate::armor::Wears)) are seeded from by value —
    /// never mutated on the roster (GTW-269 / GTW-323, mirroring the
    /// [`weapon`](GangerSpawn::weapon) key). A key absent from the registry is a handled
    /// [`BattleSetupError::ArmorNotFound`](crate::situation::BattleSetupError::ArmorNotFound)
    /// error (no panic).
    pub armor:      ArmorName,
    /// The ganger's **weapon KEY** — the filename stem of an `assets/weapons/*.ron`
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

/// One authored piece of cover — a wall *or* a scatter prop, the SAME schema for
/// both (`docs/combat/resolution.md` §3: "one ledger for walls *and* props").
///
/// A named struct carrying the authored cover facts
/// [`setup_battle`](crate::situation::setup_battle) pours into both the
/// [`CoverLedger`](crate::cover::CoverLedger) (its max [`CoverHp`] / [`HeightBand`] /
/// armor) and the [`OccupancyGrid`](crate::occupancy::OccupancyGrid) terrain (its
/// [`TerrainKind`]). Walls and scatter differ only in which [`Situation`] list they
/// live in ([`walls`](Situation::walls) vs [`scatter`](Situation::scatter)) — the
/// cover model treats them identically.
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
/// `vertical_links` extension (moved here). [`setup_battle`](crate::situation::setup_battle)
/// reads it to build the battle in the ECS world.
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
    /// The gang the human player controls — every other [`Faction`] is the enemy.
    /// Seeds the [`PlayerFaction`](crate::PlayerFaction) battle-lifetime resource the
    /// later control-gating + victory-census slices read. The struct-level
    /// `#[serde(default)]` supplies [`Faction::default`] = `Faction(0)` for any
    /// authored file that omits the field, so every existing situation `.ron` stays
    /// valid (gang `0` is the player by convention, matching `skirmish.ron`); an
    /// authored `player_faction: 1` parses as the bare gang index
    /// ([`Faction`] is `#[serde(transparent)]`).
    pub player_faction: Faction,
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
            .chain(self.slabs.iter().copied())
    }
}
