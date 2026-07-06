//! The authored `(cell, key)` placement records: [`CoverSpawn`], [`SlabSpawn`],
//! [`FloorSpawn`], and the GTW-545 [`FieldSpawn`] — each an authored `(cell, level)`
//! paired with a registry KEY resolved abort-first at setup.

use serde::{Deserialize, Serialize};

use crate::{effects::fields::FieldKey, metric::CellLevel, terrain::def::TerrainUuid};

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
/// Walls and scatter differ only in which [`Situation`](crate::situation::Situation) list they live in
/// ([`walls`](crate::situation::Situation::walls) vs [`scatter`](crate::situation::Situation::scatter)) — the cover model
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
/// [`default_floor`](crate::situation::Situation::default_floor).
///
/// GTW-396 Decision B: the [`Situation`](crate::situation::Situation) carries a sparse `floors` list for cells
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

/// One authored area-damage **field placement** — a `(cell, level)` paired with a field-type
/// KEY (GTW-545, child GTW-41f).
///
/// The seeding half of the field model: a situation `.ron` seeds a persistent hazard (a toxic
/// waste pool as initial terrain, an electrified floor) by listing a
/// [`FieldSpawn`] in [`Situation::fields`](crate::situation::Situation::fields). At [`setup_battle`](crate::situation::setup_battle)
/// the [`field`](FieldSpawn::field) KEY is resolved against the
/// [`FieldDefRegistry`](crate::effects::fields::FieldDefRegistry) (abort-first, like the terrain /
/// weapon / armor keys) and placed into the live
/// [`FieldRegistry`](crate::effects::fields::FieldRegistry) via
/// [`FieldRegistry::spawn`](crate::effects::fields::FieldRegistry::spawn).
///
/// A named struct (`at` + `field`) so the authored shape is self-describing, mirroring
/// [`CoverSpawn`]. Derives [`Deserialize`] so an authored situation `.ron` names each field's
/// `(cell, level)` + its field-type KEY (round-trips through the landed newtype serde derives —
/// render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FieldSpawn {
    /// The `(cell, level)` this field occupies.
    pub at:    CellLevel,
    /// The field-type KEY — the [`FieldKey`] of a catalog
    /// [`FieldDef`](crate::effects::fields::FieldDef), resolved against the
    /// [`FieldDefRegistry`](crate::effects::fields::FieldDefRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle). A key absent from the catalog is a
    /// handled
    /// [`BattleSetupError::FieldNotFound`](crate::situation::BattleSetupError::FieldNotFound)
    /// error (no panic).
    pub field: FieldKey,
}

impl FieldSpawn {
    /// Build an authored field placement from its `(cell, level)` and field-type KEY.
    #[must_use]
    pub const fn new(at: CellLevel, field: FieldKey) -> Self {
        Self { at, field }
    }
}
