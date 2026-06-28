//! The **prefab** schema + registry — the canonical level-fragment `.ron` format the
//! space-packing assembler (GTW-424, main game) and the editor saver (GTW-432) both
//! consume, plus the per-theme registry the game `Load` flow populates (GTW-418).
//!
//! A **prefab** is a reusable LEVEL FRAGMENT: a small footprint (a [`GridSize`]) of
//! authored geometry — walls / scatter / slabs / floor overrides / vertical links over a
//! [`default_floor`] — tagged with the [`LevelTheme`] it draws from and the
//! [`SpawnRole`] it plays in an assembled level (a player-deployment fragment, an
//! enemy-deployment fragment, or generic fill). The assembler space-packs prefabs into a
//! full [`Situation`](crate::situation::Situation); each prefab connects to its neighbours
//! through a 1-cell `default_floor` seam, so EVERY prefab must expose at least one
//! [`EdgeOpening`] — a walkable cell on a footprint boundary edge the seam can join to
//! (the C6 connectivity-by-construction ruling, validated fail-closed at load).
//!
//! # Why the schema lives sim-side
//!
//! The prefab is render-free authored DATA (the situation value-graph precedent — it
//! reuses [`CoverSpawn`] / [`SlabSpawn`] / [`FloorSpawn`] / [`VerticalLink`] /
//! [`TerrainName`] verbatim), so it lives in the sim model crate beside the situation /
//! level types. BOTH the game assembler (GTW-424) and the editor (`gdtf_editor`,
//! GTW-432) read this ONE schema, so it cannot live in either app crate.
//!
//! # The edge-opening encoding (C6)
//!
//! Edge openings are **authored explicitly** — a [`PrefabSpec`] names its
//! [`edge_openings`](PrefabSpec::edge_openings) as a list of boundary-edge open cells —
//! rather than derived by the loader inspecting which boundary cells are walkable. The
//! explicit encoding is the cleaner of the two the contract offers because:
//!
//! - The seam contract is then PART of the authored prefab, so both downstream consumers
//!   (assembler + editor) read it directly without re-deriving floor-vs-wall logic that
//!   could drift from the loader's derivation.
//! - "Which boundary cell is a deliberate doorway" is an authoring INTENT, not a fact
//!   mechanically recoverable from the floor surface (a boundary `default_floor` cell is
//!   not necessarily meant as a seam — e.g. an interior courtyard touching the edge).
//!
//! The loader counts the authored openings and rejects a prefab with ZERO of them via the
//! typed [`PrefabLoadError::NoEdgeOpening`] (no panic — fail-closed exclusion + a warn).

use bevy::{platform::collections::HashMap, prelude::Resource, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{GridSize, LevelTheme};
use crate::{
    metric::CellLevel,
    situation::{CoverSpawn, FloorSpawn, SlabSpawn},
    terrain::piece::TerrainName,
    vertical::VerticalLink,
};

/// The **role** a prefab plays in an assembled level — a closed `PlayerEnemyFill`-style
/// set the assembler buckets prefabs by (GTW-418 C3).
///
/// A named domain enum (no-bare-types: a fragment's deployment role is a domain value,
/// not a bare `u8`/string). The assembler picks a [`Player`](SpawnRole::Player) fragment
/// for the human deployment zone, an [`Enemy`](SpawnRole::Enemy) fragment for the
/// opposing one, and [`Fill`](SpawnRole::Fill) fragments for the connective interior.
/// Derives [`Deserialize`] so a prefab `.ron` names its role as a bare variant
/// (`spawn_role: Fill`), and [`Hash`]/[`Eq`] so the [`PrefabRegistry`] can key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum SpawnRole {
    /// A fragment that hosts the human player's deployment zone.
    Player,
    /// A fragment that hosts the enemy deployment zone.
    Enemy,
    /// A generic connective / interior fragment (neither deployment zone).
    Fill,
}

/// A prefab's **name** — the stable identifier the [`PrefabRegistry`] records and the
/// loader derives from each prefab file's stem (GTW-418).
///
/// A key newtype over [`String`] (no-bare-types rule 1), the
/// [`WeaponName`](crate::weapon::WeaponName) / [`TileKey`](super::TileKey) precedent.
/// Private inner + derived [`Deref`](bevy::prelude::Deref); `#[serde(transparent)]` is
/// NOT needed (a prefab file does not author its own name — the loader keys it by the
/// file stem), but the type still derives the hashing traits so it can index a registry
/// entry.
#[derive(bevy::prelude::Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrefabName(String);

impl PrefabName {
    /// Build a prefab name from its identifier string (the loader passes the file stem).
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// One **edge opening** — a walkable cell on a prefab footprint's boundary edge through
/// which the inter-prefab 1-cell `default_floor` seam connects (GTW-418 C6).
///
/// A named newtype over a [`CellLevel`] (no-bare-types: a seam doorway is a domain value,
/// not a bare grid key) so an opening is never confused with an arbitrary authored cell.
/// The assembler joins a neighbouring fragment's seam to this cell; a prefab with ZERO
/// openings cannot connect and is rejected at load ([`PrefabLoadError::NoEdgeOpening`]).
/// Private inner + derived [`Deref`](bevy::prelude::Deref); deserializes through the
/// [`CellLevel`] `(cell, level)` authoring shape via the [`EdgeOpeningDef`] intermediate,
/// and serializes back out through the SAME shape (`#[serde(into = "EdgeOpeningDef")]`),
/// so the editor prefab saver (GTW-432) writes an opening that round-trips byte-for-byte
/// through the `from = "EdgeOpeningDef"` reader.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "EdgeOpeningDef", into = "EdgeOpeningDef")]
pub struct EdgeOpening(CellLevel);

impl EdgeOpening {
    /// Build an edge opening at a `(cell, level)` boundary cell.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }

    /// The `(cell, level)` boundary cell this opening sits on.
    #[must_use]
    pub const fn at(self) -> CellLevel {
        self.0
    }
}

/// The authored RON shape an [`EdgeOpening`] deserializes from — a single `at`
/// `(cell, level)`, routed through [`EdgeOpening::new`] (GTW-418).
///
/// A serde intermediate (the [`CellLevelDef`](crate::metric) precedent) so an authored
/// opening writes `(at: (cell: .., level: ..))` and keeps the newtype inner private
/// across (de)serialization — read in through `from`, written back out through `into`.
#[derive(Deserialize, Serialize)]
pub struct EdgeOpeningDef {
    /// The `(cell, level)` boundary cell the opening sits on.
    at: CellLevel,
}

impl From<EdgeOpeningDef> for EdgeOpening {
    fn from(def: EdgeOpeningDef) -> Self {
        Self::new(def.at)
    }
}

impl From<EdgeOpening> for EdgeOpeningDef {
    fn from(opening: EdgeOpening) -> Self {
        Self { at: opening.at() }
    }
}

/// The **authoring struct** a `assets/content/maps/<theme>/<size>/<prefab>.ron`
/// deserializes into — one reusable level fragment (GTW-418).
///
/// The canonical prefab schema BOTH the GTW-424 assembler and the GTW-432 editor saver
/// consume. It reuses the situation value-graph leaves verbatim
/// ([`CoverSpawn`] for walls + scatter, [`SlabSpawn`] for slabs, [`FloorSpawn`] for
/// per-cell floor overrides, [`VerticalLink`] for ladders/stairs, [`TerrainName`] for the
/// default floor), so a fragment is authored exactly like a small standalone situation —
/// plus two prefab-specific fields: the [`spawn_role`](PrefabSpec::spawn_role) and the
/// [`edge_openings`](PrefabSpec::edge_openings) the seam connects through.
///
/// Every authored magnitude (terrain HP / move cost behind each [`TerrainName`] key) is
/// tuning DATA resolved later against the registries — NOT pinned by tests
/// (the brittle-test rule). Derives [`Deserialize`] so the `.ron` parses and
/// [`TypePath`] because `RonAsset<PrefabSpec>` requires its payload to be [`TypePath`]
/// (the [`ThemeSpec`](super::ThemeSpec) precedent).
///
/// **Not `Copy`** — it owns several `Vec`s + `String`-backed [`TerrainName`]; it is
/// `Clone` so the registry can hold prefabs by value.
///
/// Derives [`Serialize`] (GTW-432) so the editor saver writes a `.prefab.ron` in EXACTLY
/// this schema — the same type the GTW-418 loader deserializes — so `load(save(grid))`
/// round-trips with no data loss. Every leaf in the value graph
/// ([`LevelTheme`] / [`GridSize`] / [`SpawnRole`] / [`TerrainName`] / [`CoverSpawn`] /
/// [`SlabSpawn`] / [`FloorSpawn`] / [`VerticalLink`] / [`EdgeOpening`]) serializes through
/// the same authoring shape it deserializes from, so a written prefab parses straight back.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
#[serde(default)]
pub struct PrefabSpec {
    /// The [`LevelTheme`] this fragment draws from — the registry's primary enumeration
    /// key (the assembler picks fragments matching the level's theme). `#[serde(default)]`
    /// supplies [`LevelTheme::default`] for a file that omits it.
    pub theme:          LevelTheme,
    /// The fragment's footprint dimensions (its [`GridSize`] — width × height ×
    /// storey-count, each axis validated to the sim coarse-grid maximum). A registry
    /// enumeration key. `#[serde(default)]` supplies the full sim extent for a file that
    /// omits it (the [`Situation::grid_size`](crate::situation::Situation) precedent).
    pub size:           GridSize,
    /// The [`SpawnRole`] this fragment plays — the third registry enumeration key.
    /// `#[serde(default)]`-absent files are uncommon (the role is the point of a prefab),
    /// so there is NO `Default` for [`SpawnRole`]; an authored file MUST name it. (The
    /// struct-level `#[serde(default)]` only fills OMITTED fields from
    /// [`PrefabSpec::default`], which names [`SpawnRole::Fill`] — see the [`Default`] impl.)
    pub spawn_role:     SpawnRole,
    /// The default floor terrain piece KEY filling every walkable open cell of the
    /// footprint (the [`Situation::default_floor`](crate::situation::Situation)
    /// precedent — the seam cells are this floor). `#[serde(default)]` supplies the empty
    /// sentinel.
    pub default_floor:  TerrainName,
    /// The authored walls (each a [`CoverSpawn`] — `at` + terrain piece KEY).
    pub walls:          Vec<CoverSpawn>,
    /// The authored scatter / props (each a [`CoverSpawn`], same schema as a wall).
    pub scatter:        Vec<CoverSpawn>,
    /// The authored floor / roof slabs (each a [`SlabSpawn`] — `at` + terrain piece KEY,
    /// for multi-level fragments).
    pub slabs:          Vec<SlabSpawn>,
    /// Sparse per-cell floor overrides (cells whose floor differs from
    /// [`default_floor`](PrefabSpec::default_floor)).
    pub floors:         Vec<FloorSpawn>,
    /// The authored stair / ladder vertical links (multi-level fragments) — the only way
    /// a ganger changes storey within the fragment.
    pub vertical_links: Vec<VerticalLink>,
    /// The walkable boundary-edge cells the inter-prefab seam connects through (C6). A
    /// prefab with ZERO of these is rejected at load ([`PrefabLoadError::NoEdgeOpening`]),
    /// so a valid prefab always authors `>= 1`. `#[serde(default)]` gives an empty list,
    /// which the loader then REJECTS — the empty-but-present case is the fail-closed one,
    /// not a parse error.
    pub edge_openings:  Vec<EdgeOpening>,
}

impl Default for PrefabSpec {
    /// An empty [`Fill`](SpawnRole::Fill) prefab at the default footprint — the
    /// `#[serde(default)]` source for OMITTED fields only (a real prefab authors a role +
    /// at least one edge opening; this default's empty `edge_openings` is itself rejected
    /// by the loader, which is correct — an unauthored prefab is not a valid one).
    fn default() -> Self {
        Self {
            theme:          LevelTheme::default(),
            size:           GridSize::default(),
            spawn_role:     SpawnRole::Fill,
            default_floor:  TerrainName::default(),
            walls:          Vec::new(),
            scatter:        Vec::new(),
            slabs:          Vec::new(),
            floors:         Vec::new(),
            vertical_links: Vec::new(),
            edge_openings:  Vec::new(),
        }
    }
}

impl PrefabSpec {
    /// How many [`EdgeOpening`]s this prefab authors — the count
    /// [`validate`](PrefabSpec::validate) requires to be `>= 1` (C6).
    #[must_use]
    pub const fn edge_opening_count(&self) -> usize {
        self.edge_openings.len()
    }

    /// Validate the C6 connectivity-by-construction invariant: a prefab must author at
    /// least ONE [`EdgeOpening`] so the inter-prefab seam can connect to it.
    ///
    /// Succeeds (unit) when the prefab is connectable, or returns a [`PrefabLoadError`]
    /// naming the offending prefab. Fail-closed with NO panic (the no-panic contract): a
    /// zero-opening prefab is REJECTED here, and the loader excludes it and warns rather
    /// than crashing.
    ///
    /// # Errors
    ///
    /// [`PrefabLoadError::NoEdgeOpening`] if the prefab authors zero edge openings.
    pub fn validate(&self, name: &PrefabName) -> Result<(), PrefabLoadError> {
        if self.edge_openings.is_empty() {
            return Err(PrefabLoadError::NoEdgeOpening(name.clone()));
        }
        Ok(())
    }
}

/// Why a prefab `.ron` was rejected at load — the fail-closed, no-panic error of the
/// prefab validation (GTW-418 C6).
///
/// The handled rejection reason (no-bare-types: the failure is a domain value, not a bare
/// `()`/`bool`; the no-panic contract — the loader excludes the prefab + warns rather than
/// `unwrap`/`panic`). The [`GridSizeError`](super::GridSizeError) precedent: each variant
/// names what was wrong so a caller can log exactly which prefab failed and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefabLoadError {
    /// The prefab authored ZERO [`EdgeOpening`]s, so the inter-prefab seam cannot connect
    /// to it — connectivity-by-construction violated (C6). Names the rejected prefab.
    NoEdgeOpening(PrefabName),
}

impl std::fmt::Display for PrefabLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEdgeOpening(name) => write!(
                f,
                "prefab `{}` has zero edge openings; every prefab must author at least one \
                 walkable boundary-edge cell for the inter-prefab seam to connect to",
                **name,
            ),
        }
    }
}

impl std::error::Error for PrefabLoadError {}

/// One **validated** prefab the [`PrefabRegistry`] holds — its [`PrefabName`] paired with
/// the authored [`PrefabSpec`] (GTW-418).
///
/// Built only through [`Prefab::new`], which runs [`PrefabSpec::validate`] (C6) — so a
/// `Prefab` in the registry ALWAYS has at least one edge opening (a zero-opening spec is
/// rejected before it becomes a `Prefab`). Holds the spec BY VALUE so it survives the
/// loaded folder handle being dropped. `Clone` (it owns the spec); not `Copy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefab {
    /// The prefab's name (the file stem the loader keyed it by).
    name: PrefabName,
    /// The validated authored spec.
    spec: PrefabSpec,
}

impl Prefab {
    /// Build a validated prefab from its name + spec, running the C6 edge-opening
    /// validation.
    ///
    /// # Errors
    ///
    /// [`PrefabLoadError::NoEdgeOpening`] if the spec authors zero edge openings — the
    /// loader excludes it + warns (fail-closed, no panic).
    pub fn new(name: PrefabName, spec: PrefabSpec) -> Result<Self, PrefabLoadError> {
        spec.validate(&name)?;
        Ok(Self { name, spec })
    }

    /// This prefab's name.
    #[must_use]
    pub const fn name(&self) -> &PrefabName {
        &self.name
    }

    /// This prefab's validated authored spec.
    #[must_use]
    pub const fn spec(&self) -> &PrefabSpec {
        &self.spec
    }
}

/// The registry **enumeration key** — the `(theme, size, spawn_role)` triple the
/// assembler lists prefabs by (GTW-418 C3).
///
/// A named struct (no-bare-types: the lookup key is a domain value, not a bare tuple) so
/// the [`PrefabRegistry`] bucket key is self-describing. All three fields are `Copy` +
/// `Hash` + `Eq` ([`LevelTheme`] / [`GridSize`] / [`SpawnRole`] all are), so the key is a
/// cheap copyable `HashMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefabKey {
    /// The theme the prefab draws from.
    pub theme:      LevelTheme,
    /// The prefab's footprint dimensions.
    pub size:       GridSize,
    /// The deployment role the prefab plays.
    pub spawn_role: SpawnRole,
}

impl PrefabKey {
    /// Build a `(theme, size, spawn_role)` enumeration key.
    #[must_use]
    pub const fn new(theme: LevelTheme, size: GridSize, spawn_role: SpawnRole) -> Self {
        Self {
            theme,
            size,
            spawn_role,
        }
    }
}

/// The **prefab registry** — the per-`(theme, size, spawn_role)` bucketed map of
/// validated prefabs the game `Load` flow builds from `assets/content/maps/` and the
/// assembler enumerates (GTW-418 C2 / C3).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`PrefabKey`]`, Vec<`[`Prefab`]`>>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), the
/// [`WeaponRegistry`](crate::weapon::WeaponRegistry) shape. Multiple prefabs can share one
/// `(theme, size, role)` key (the assembler picks among them), so each key maps to a `Vec`.
/// The sim OWNS the prefab model, so the type lives here; the app's `Load` flow POPULATES
/// it from the loaded `assets/content/maps/**/*.ron` folder and inserts it as a resource.
/// It holds the prefabs BY VALUE ([`Prefab`] is `Clone`), so they survive the loaded-folder
/// asset handle being dropped.
///
/// Private inner with named accessors (the registry answers a prefab ENUMERATION question,
/// not a raw-map one — so no derived [`Deref`](bevy::prelude::Deref)). The assembler lists
/// candidates via [`prefabs_for`](PrefabRegistry::prefabs_for).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefabRegistry(HashMap<PrefabKey, Vec<Prefab>>);

impl PrefabRegistry {
    /// Insert one validated [`Prefab`] under its `(theme, size, spawn_role)` key — the
    /// per-file insert the folder loader calls as it iterates the loaded folder.
    ///
    /// Buckets by the prefab's own [`theme`](PrefabSpec::theme) / [`size`](PrefabSpec::size)
    /// / [`spawn_role`](PrefabSpec::spawn_role), appending to any existing prefabs at that
    /// key (multiple prefabs may share one key — the assembler picks among them).
    pub fn insert(&mut self, prefab: Prefab) {
        let key = PrefabKey::new(
            prefab.spec().theme,
            prefab.spec().size,
            prefab.spec().spawn_role,
        );
        self.0.entry(key).or_default().push(prefab);
    }

    /// List every prefab registered for a `(theme, size, spawn_role)` — the assembler's
    /// enumeration query (C3). Empty (an empty slice) if none match.
    #[must_use]
    pub fn prefabs_for(&self, key: &PrefabKey) -> &[Prefab] {
        self.0.get(key).map_or(&[], Vec::as_slice)
    }

    /// How many prefabs the registry holds across every key — the count the folder-load
    /// test asserts is non-empty.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }

    /// Whether the registry holds no prefabs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }

    /// Iterate over every `(theme, size, spawn_role)` key the registry has prefabs for —
    /// for the editor's prefab-browser enumeration, so it can list available buckets
    /// without exposing the inner map. Iteration order is unspecified.
    pub fn keys(&self) -> impl Iterator<Item = &PrefabKey> {
        self.0.keys()
    }
}

#[cfg(test)]
mod test {
    use super::{
        EdgeOpening, Prefab, PrefabKey, PrefabLoadError, PrefabName, PrefabRegistry, PrefabSpec,
        SpawnRole,
    };
    use crate::{
        level::{GridHeight, GridLevels, GridSize, GridWidth, LevelTheme},
        metric::{Cell, CellLevel, Level},
    };

    /// A `(cell, level)` at the given ground coords on level 0 (a tiny helper).
    fn at(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// A 3x3x1 footprint (a valid small prefab size).
    fn small_size() -> Option<GridSize> {
        GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
    }

    /// T2: a prefab with >= 1 edge opening PASSES validation and becomes a `Prefab`.
    ///
    /// Discriminating: removing the `>= 1` check would make the zero-opening sibling test
    /// (below) ALSO pass, so this pair pins the validator both ways.
    #[test]
    fn prefab_with_edge_opening_validates() {
        let Some(size) = small_size() else { return };
        let spec = PrefabSpec {
            size,
            edge_openings: vec![EdgeOpening::new(at(2, 1))],
            ..PrefabSpec::default()
        };

        let name = PrefabName::new("entry_room".to_owned());
        let built = Prefab::new(name, spec);
        assert!(
            built.is_ok(),
            "a prefab authoring >= 1 edge opening must validate: {:?}",
            built.as_ref().err(),
        );
    }

    /// T2 (discriminating sibling): a prefab with ZERO edge openings is REJECTED
    /// fail-closed with the typed `NoEdgeOpening` error — no panic (C6).
    ///
    /// Must FAIL if the `>= 1` validation were removed (the build would then be `Ok`).
    #[test]
    fn prefab_without_edge_opening_is_rejected_fail_closed() {
        let Some(size) = small_size() else { return };
        // No edge openings authored (the Default's empty list) — the fail-closed case.
        let spec = PrefabSpec {
            size,
            edge_openings: Vec::new(),
            ..PrefabSpec::default()
        };

        let name = PrefabName::new("sealed_box".to_owned());
        let built = Prefab::new(name.clone(), spec);
        assert_eq!(
            built.err(),
            Some(PrefabLoadError::NoEdgeOpening(name)),
            "a zero-edge-opening prefab must be rejected with the typed NoEdgeOpening error",
        );
    }

    /// C3: the registry enumerates prefabs by `(theme, size, spawn_role)`. Insert two
    /// prefabs differing only in role and assert each is listed under its own key and not
    /// the other's.
    #[test]
    fn registry_enumerates_by_theme_size_role() {
        let Some(size) = small_size() else { return };
        let mut registry = PrefabRegistry::default();

        for (stem, role) in [
            ("player_pad", SpawnRole::Player),
            ("fill_hall", SpawnRole::Fill),
        ] {
            let spec = PrefabSpec {
                theme: LevelTheme::IndustrialHive,
                size,
                spawn_role: role,
                edge_openings: vec![EdgeOpening::new(at(0, 1))],
                ..PrefabSpec::default()
            };
            if let Ok(prefab) = Prefab::new(PrefabName::new(stem.to_owned()), spec) {
                registry.insert(prefab);
            }
        }

        let player_key = PrefabKey::new(LevelTheme::IndustrialHive, size, SpawnRole::Player);
        let fill_key = PrefabKey::new(LevelTheme::IndustrialHive, size, SpawnRole::Fill);
        let enemy_key = PrefabKey::new(LevelTheme::IndustrialHive, size, SpawnRole::Enemy);

        assert_eq!(
            registry.prefabs_for(&player_key).len(),
            1,
            "one Player prefab"
        );
        assert_eq!(registry.prefabs_for(&fill_key).len(), 1, "one Fill prefab");
        assert!(
            registry.prefabs_for(&enemy_key).is_empty(),
            "no Enemy prefab was inserted",
        );
        assert_eq!(registry.len(), 2, "two prefabs total across keys");
        assert_eq!(
            registry
                .prefabs_for(&player_key)
                .first()
                .map(|p| (**p.name()).clone()),
            Some("player_pad".to_owned()),
            "the Player bucket holds the player_pad prefab",
        );
    }
}
