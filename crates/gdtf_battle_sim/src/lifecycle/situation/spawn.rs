//! The authored value graph: [`PlacedGanger`], [`GangerSpawn`], [`CoverSpawn`],
//! [`SlabSpawn`], [`FloorSpawn`], and the canonical [`Situation`] — the
//! serde-deserializable battlefield the setup is built from.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{
    ganger::{
        Aim, Aiming, Cool, Facing, Faction, GangMember, GangName, GangerName, Grit, LifeState,
        Luck, Reflexes, Speed, Stance, Strength, Toughness,
    },
    level::{GridSize, ThemeUuid},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    vertical::VerticalLink,
    weapon::WeaponName,
};

/// One **placed ganger** — the GTW-414 v2 `Situation`-side ganger reference: WHICH
/// roster member fights, WHERE, and on WHICH side.
///
/// The placement half of the schema-v2 split (GTW-414). It carries NO identity /
/// attributes / equipment of its own — those come from the gang roster: it references
/// its [`gang`](PlacedGanger::gang) ([`GangName`], a [`GangRegistry`](crate::ganger::GangRegistry)
/// key) and the [`member`](PlacedGanger::member) within it (by [`GangerName`]), and
/// [`setup_battle`](crate::situation::setup_battle) resolves `(gang, member)` to a
/// [`GangMember`](crate::ganger::GangMember) (eight attributes + weapon + armor keys). The
/// `Situation` supplies the rest: the `(cell, level)` placement
/// ([`at`](PlacedGanger::at)), the posture / facing / aim / life fields, and the
/// [`faction`](PlacedGanger::faction) the ganger fights for in THIS battle — so the same
/// faction-agnostic gang roster can be fielded on any side at any position.
///
/// `PartialEq` + `Eq` (every field — the gang/member names, the `(cell, level)`, and the
/// fieldless posture enums — has a total `Eq`, unlike the f32-bearing roster
/// [`GangMember`](crate::ganger::GangMember)). Not `Hash`: `(cell, level)`-keyed
/// de-duplication ([`has_stacked_gangers`](crate::situation::has_stacked_gangers)) hashes
/// [`at`](PlacedGanger::at), never the whole struct. `Clone` (owned `String`-backed
/// names). Derives [`Deserialize`] so an authored situation `.ron` names
/// each placement as a self-describing record (the value graph all flows through the
/// landed newtype/enum serde derives — render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlacedGanger {
    /// The **gang** this ganger's roster comes from — a [`GangName`] resolved against the
    /// [`GangRegistry`](crate::ganger::GangRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle). A gang absent from the registry
    /// is a handled
    /// [`BattleSetupError::GangNotFound`](crate::situation::BattleSetupError::GangNotFound)
    /// error (no panic). Authored as a bare string ([`GangName`] is `#[serde(transparent)]`).
    pub gang:       GangName,
    /// The **member** within the gang this ganger IS — a [`GangerName`] resolved against
    /// the gang's roster at setup (`gang.member(member)`). A member absent from the gang
    /// is a handled
    /// [`BattleSetupError::GangMemberNotFound`](crate::situation::BattleSetupError::GangMemberNotFound)
    /// error (no panic). Authored as a bare string ([`GangerName`] is `#[serde(transparent)]`).
    pub member:     GangerName,
    /// The `(cell, level)` the ganger spawns at — its [`Position`](crate::ganger::Position).
    pub at:         CellLevel,
    /// The ganger's gang (faction) identity for THIS battle — the side assignment the
    /// situation makes (NOT part of the roster; the roster is faction-agnostic).
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
}

impl PlacedGanger {
    /// Build a placed ganger from its gang/member refs and a [`Placement`] — the public
    /// constructor (house style) so the setup, the test builders, and the presenter can
    /// build one without reaching the fields piecemeal.
    ///
    /// The six situation-supplied placement fields (`at` / `faction` / `facing` / `stance`
    /// / `aiming` / `life_state`) are grouped into the named [`Placement`] argument (a real
    /// named type per no-bare-types, NOT a tuple), keeping the constructor's parameter list
    /// under clippy's argument-count gate while the struct itself stays flat for serde.
    #[must_use]
    pub const fn new(gang: GangName, member: GangerName, placement: Placement) -> Self {
        Self {
            gang,
            member,
            at: placement.at,
            faction: placement.faction,
            facing: placement.facing,
            stance: placement.stance,
            aiming: placement.aiming,
            life_state: placement.life_state,
        }
    }
}

/// The six **situation-supplied placement fields** a [`PlacedGanger`] carries — grouped
/// into one named type so [`PlacedGanger::new`] takes a single placement argument instead
/// of six positional ones (no-bare-types: a real named grouping struct, never a tuple).
///
/// This is the WHERE + WHICH-SIDE half of the GTW-414 schema-v2 split: the cell/level the
/// ganger spawns at, the [`faction`](Placement::faction) it fights for THIS battle, and its
/// posture / facing / aim / life fields. The roster half (identity + attributes + weapon /
/// armor) comes from the gang [`GangMember`](crate::ganger::GangMember), not here. A pure
/// in-code constructor helper — it is NOT itself authored: the situation `.ron` authors
/// these as flat fields on the [`PlacedGanger`] record, and the test builders /
/// [`GangerSpawn::split`] assemble a `Placement` to call the constructor.
///
/// All fields are `Copy`, so the struct is `Copy` (a cheap by-value placement bundle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// The `(cell, level)` the ganger spawns at — its [`Position`](crate::ganger::Position).
    pub at:         CellLevel,
    /// The faction (side) the ganger fights for in THIS battle (placement, not roster).
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
}

impl Placement {
    /// Build a placement bundle from its six situation-supplied fields — the shape
    /// [`GangerSpawn::split`] and the test builders assemble to call
    /// [`PlacedGanger::new`].
    #[must_use]
    pub const fn new(
        at: CellLevel,
        faction: Faction,
        facing: Facing,
        stance: Stance,
        aiming: Aiming,
        life_state: LifeState,
    ) -> Self {
        Self {
            at,
            faction,
            facing,
            stance,
            aiming,
            life_state,
        }
    }
}

/// A combined **roster-plus-placement** authoring record — the GTW-414 schema-v2
/// authoring HELPER that the test builders and the migration assemble, then SPLIT into a
/// [`PlacedGanger`] (the `Situation`-side placement + faction) and a
/// [`GangMember`](crate::ganger::GangMember) (the gang-roster identity + attributes +
/// equipment) via [`split`](GangerSpawn::split).
///
/// Before GTW-414 this WAS the `Situation.gangers` element (roster + placement + faction
/// in one struct). The v2 schema splits roster (the reusable, faction-agnostic gang
/// member) from placement (where + which side the situation assigns), so the canonical
/// `Situation` now holds [`PlacedGanger`]s and the gang rosters live in a
/// [`GangRegistry`](crate::ganger::GangRegistry). This combined record survives ONLY as
/// the ergonomic authoring shape (one literal carries everything about one fielded
/// ganger), and [`split`](GangerSpawn::split) decomposes it into the two v2 halves.
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
    /// The ganger's **weapon KEY** — the filename stem of an `assets/content/weapons/ranged/*.ron`
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

impl GangerSpawn {
    /// Split this combined authoring record into its GTW-414 schema-v2 halves: a
    /// [`PlacedGanger`] (the `Situation`-side placement + faction) referencing the given
    /// `gang`, paired with the [`GangMember`](crate::ganger::GangMember) (the gang-roster
    /// identity + eight attributes + weapon / armor keys) it points at.
    ///
    /// The placement's [`member`](PlacedGanger::member) is this ganger's
    /// [`name`](GangerSpawn::name), so the returned `PlacedGanger.member` resolves against
    /// the returned `GangMember` in the registry. The test builders and the migration use
    /// this to assemble a v2 [`Situation`] + [`GangRegistry`](crate::ganger::GangRegistry)
    /// pair from the ergonomic combined literal.
    #[must_use]
    pub fn split(&self, gang: GangName) -> (PlacedGanger, GangMember) {
        let placed = PlacedGanger::new(
            gang,
            self.name.clone(),
            Placement::new(
                self.at,
                self.faction,
                self.facing,
                self.stance,
                self.aiming,
                self.life_state,
            ),
        );
        let member = GangMember {
            name:         self.name.clone(),
            speed:        self.speed,
            aim:          self.aim,
            strength:     self.strength,
            toughness:    self.toughness,
            reflexes:     self.reflexes,
            cool:         self.cool,
            grit:         self.grit,
            luck:         self.luck,
            armor:        self.armor.clone(),
            weapon:       self.weapon.clone(),
            // GTW-505: the combined `GangerSpawn` authors no melee weapon — the split
            // member gets `None`, which `setup_battle` resolves to the shipped `fists`
            // default (every ganger can melee). A future builder method can override it.
            melee_weapon: None,
        };
        (placed, member)
    }
}

/// One authored piece of cover — a wall *or* a scatter prop.
///
/// GTW-491 migration (child T07a of the GTW-476 data-model refactor): the
/// [`piece`](CoverSpawn::piece) field switches from the legacy filename-stem
/// [`TerrainName`](crate::terrain::piece::TerrainName) key to the UUID-keyed
/// [`TerrainUuid`], resolved against the
/// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) at setup. The kind
/// ([`TerrainKind`](crate::occupancy::TerrainKind) / `TerrainPieceKind`) and structural
/// stats ([`CoverEntry`](crate::cover::CoverEntry)) are DERIVED from the resolved
/// definition's [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) variant at
/// `setup_battle` time (a `Wall` def → `TerrainKind::Wall`; a `Cover` def →
/// `TerrainKind::Cover`).
///
/// A named struct (`at` + `piece`) so the authored shape is self-describing.
/// Derives [`Deserialize`] so an authored situation `.ron` can name each piece's
/// `(cell, level)` + its terrain UUID (round-trips through the landed newtype
/// serde derives — render-free, pixel-free).
///
/// Walls and scatter differ only in which [`Situation`] list they live in
/// ([`walls`](Situation::walls) vs [`scatter`](Situation::scatter)) — the cover model
/// treats them identically.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CoverSpawn {
    /// The `(cell, level)` this cover piece occupies.
    pub at:    CellLevel,
    /// The terrain definition KEY — the stable [`TerrainUuid`] of a migrated
    /// [`TerrainDef`](crate::terrain::def::TerrainDef), resolved against the
    /// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle). A key absent from the registry
    /// is a handled
    /// [`BattleSetupError::TerrainNotFound`](crate::situation::BattleSetupError::TerrainNotFound)
    /// error (no panic).
    pub piece: TerrainUuid,
}

impl CoverSpawn {
    /// Build an authored cover piece from its `(cell, level)` and terrain UUID key.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid) -> Self {
        Self { at, piece }
    }
}

/// One authored floor / roof slab — its `(cell, level)` and a terrain definition KEY.
///
/// GTW-491 migration (T07a): the [`piece`](SlabSpawn::piece) field switches from the
/// legacy filename-stem [`TerrainName`](crate::terrain::piece::TerrainName) key to the
/// UUID-keyed [`TerrainUuid`], resolved against the
/// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) at setup. The slab's
/// per-slab structural HP/armor are seeded from the resolved definition's
/// [`TerrainSimKind::Slab`](crate::terrain::def::TerrainSimKind::Slab) variant.
///
/// Derives [`Deserialize`] so an authored situation `.ron` writes each slab as
/// `(at: (cell: …, level: …), piece: "…")` — the same shape as [`CoverSpawn`], but
/// into the `slabs` list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SlabSpawn {
    /// The `(cell, level)` this slab occupies (same meaning as the old bare entry).
    pub at:    CellLevel,
    /// The terrain definition KEY — the stable [`TerrainUuid`] of a migrated
    /// [`TerrainDef`](crate::terrain::def::TerrainDef), resolved against the
    /// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle) into the
    /// [`TerrainSimKind::Slab`](crate::terrain::def::TerrainSimKind::Slab) stats that seed
    /// this slab's structural HP/armor in both the
    /// [`SlabLedger`](crate::slab::SlabLedger) and the spawned terrain entity
    /// (GTW-395/396 — per-slab authored HP, not uniform tuning). A key absent from
    /// the registry is a handled
    /// [`BattleSetupError::TerrainNotFound`](crate::situation::BattleSetupError::TerrainNotFound)
    /// error (no panic).
    pub piece: TerrainUuid,
}

impl SlabSpawn {
    /// Build a slab spawn from its cell + terrain UUID key.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid) -> Self {
        Self { at, piece }
    }
}

/// One authored per-cell floor override — a `(cell, level)` paired with a terrain
/// definition KEY that gives it a different floor than the situation's
/// [`default_floor`](Situation::default_floor).
///
/// GTW-396 Decision B: the [`Situation`] carries a sparse `floors` list for cells
/// whose floor cost deviates from the default. An empty `floors` list (the common
/// case — a uniform floor) is the cheapest authored state and the `#[serde(default)]`
/// for this field.
///
/// GTW-491 migration (T07a): the [`piece`](FloorSpawn::piece) field switches from the
/// legacy [`TerrainName`](crate::terrain::piece::TerrainName) key to the UUID-keyed
/// [`TerrainUuid`]. (The new [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) model
/// has no `Floor` variant — a walkable floor is a `Slab` def; the per-cell move-cost seam is
/// GTW-482, so this slice carries the reference forward without resolving its move cost.)
///
/// Derives [`Deserialize`] so an authored situation `.ron` writes each override as
/// `(at: (cell: …, level: …), piece: "…")`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FloorSpawn {
    /// The `(cell, level)` with a non-default floor.
    pub at:    CellLevel,
    /// The terrain definition KEY — the stable [`TerrainUuid`] giving this cell its floor.
    pub piece: TerrainUuid,
}

impl FloorSpawn {
    /// Build a floor spawn from its cell + terrain UUID key.
    #[must_use]
    pub const fn new(at: CellLevel, piece: TerrainUuid) -> Self {
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
/// # The roster-vs-placement contract (GTW-414 schema v2)
///
/// The v2 schema SPLITS what an authored ganger carries between the **gang roster** (the
/// reusable, faction-agnostic identity + abilities + equipment) and the **situation**
/// (where + which side it fights on):
///
/// - **Lives in the gang roster** ([`GangMember`](crate::ganger::GangMember), in a
///   `*.gang.ron`, resolved via the [`GangRegistry`](crate::ganger::GangRegistry)): the
///   member's [`GangerName`] identity, its EIGHT direct attributes (Speed / Aim /
///   Strength / Toughness / Reflexes / Cool / Grit / Luck), its weapon KEY, and its armor
///   KEY. A gang roster says NOTHING about position or faction, so the SAME roster is
///   reusable in any battle, on any side, at any cell.
/// - **The situation assigns** ([`PlacedGanger`], in this `Situation`): WHICH gang +
///   member fights ([`gang`](PlacedGanger::gang) + [`member`](PlacedGanger::member)
///   refs), WHERE ([`at`](PlacedGanger::at) + [`facing`](PlacedGanger::facing) /
///   [`stance`](PlacedGanger::stance) / [`aiming`](PlacedGanger::aiming) /
///   [`life_state`](PlacedGanger::life_state)), and WHICH SIDE
///   ([`faction`](PlacedGanger::faction) — faction is placement/assignment, NOT roster).
///
/// [`setup_battle`](crate::situation::setup_battle) joins the two: for each
/// [`PlacedGanger`] it resolves `(gang, member)` against the
/// [`GangRegistry`](crate::ganger::GangRegistry) to get the roster member, derives the
/// computed combat stats from its eight attributes, resolves its weapon / armor keys, and
/// spawns the ganger at the situation-supplied placement + faction.
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
    /// The authored placed gangers (GTW-414 schema v2), each a [`PlacedGanger`]: a gang +
    /// member REF (resolved against the [`GangRegistry`](crate::ganger::GangRegistry) at
    /// setup for identity / attributes / equipment) plus the situation-supplied placement
    /// (`at` / `facing` / `stance` / `aiming` / `life_state`) and `faction`.
    pub gangers:        Vec<PlacedGanger>,
    /// The level THEME this battlefield draws from — the stable UUID-keyed reference to a
    /// migrated [`UuidThemeDef`](crate::level::UuidThemeDef), resolvable in the
    /// [`UuidThemeRegistry`](crate::level::UuidThemeRegistry).
    ///
    /// GTW-491 migration (T07a): switched from the closed-enum theme model to the UUID-keyed
    /// [`ThemeUuid`] (reconciling the GTW-490 additive
    /// `theme_uuid` field — the canonical sim theme is now this one `theme`). `#[serde(default)]`
    /// supplies [`ThemeUuid::default`] (the nil sentinel) for any authored file that omits the
    /// field. An authored `theme: "<uuid>"` parses the `#[serde(transparent)]` [`ThemeUuid`]
    /// string wire form.
    pub theme:          ThemeUuid,
    /// The coarse GRID dimensions of this battlefield (GTW-414) — width × height ×
    /// storey-count, each axis validated to the sim's coarse-grid maximum (GTW-409
    /// [`GridSize`]).
    ///
    /// `#[serde(default)]` supplies [`GridSize::default`] = the FULL documented `60×60×8`
    /// extent for any authored file that omits the field, so every PRE-GTW-414 situation
    /// `.ron` stays parse-valid — the full sim extent is the safe superset of any cell a
    /// pre-GTW-414 file authored. An authored
    /// `grid_size: (width: N, height: N, levels: N)` flows through the bounds-checked
    /// [`GridSize`] serde intermediate, so an over-max / empty grid can never deserialize.
    pub grid_size:      GridSize,
    /// The authored walls (each a [`CoverSpawn`] — `at` + piece KEY).
    pub walls:          Vec<CoverSpawn>,
    /// The authored scatter / props (each a [`CoverSpawn`], same schema as a wall).
    pub scatter:        Vec<CoverSpawn>,
    /// The authored floor / roof slabs (each a [`SlabSpawn`] — `at` + piece KEY).
    /// GTW-396: was `Vec<CellLevel>`; now `Vec<SlabSpawn>` so each slab carries its
    /// terrain piece KEY for per-slab HP/armor resolution against the
    /// terrain-definition registry.
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
    /// The default floor terrain definition KEY — the stable [`TerrainUuid`] of a migrated
    /// walkable [`TerrainDef`](crate::terrain::def::TerrainDef) (a `Slab`-kind floor in the new
    /// model). Applied to every walkable open cell not overridden by [`floors`](Situation::floors).
    ///
    /// GTW-491 migration (T07a): switched from the legacy filename-stem
    /// [`TerrainName`](crate::terrain::piece::TerrainName) key to the UUID-keyed [`TerrainUuid`].
    /// `#[serde(default)]` supplies the NIL sentinel ([`TerrainUuid::default`] —
    /// [`TerrainUuid::is_nil`]) for any authored file that omits the field, so every EXISTING
    /// situation `.ron` remains parse-valid. A nil default skips registry floor resolution (the
    /// sim falls back to seeding the [`FloorCostGrid`](crate::terrain::floor::FloorCostGrid) from
    /// `CombatTuning::move_costs.open`, preserving the pre-GTW-396 behavior for test fixtures and
    /// un-migrated situations).
    pub default_floor:  TerrainUuid,
    /// Sparse per-cell floor piece overrides — cells whose floor differs from
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
