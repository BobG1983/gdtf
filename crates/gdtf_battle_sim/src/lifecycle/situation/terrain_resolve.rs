//! The GTW-396 terrain-resolution cluster — the pure helpers
//! [`setup_battle`](super::setup_battle) calls to turn each authored terrain piece KEY
//! (cover / slab / floor) into a resolved spec BEFORE any entity is spawned
//! (abort-first), kept out of `setup.rs` so the spawn orchestration stays
//! single-responsibility.
//!
//! Every helper here is registry-read + validation only — no `Commands`, no spawn, no
//! world mutation. They return the resolved [`ResolvedCoverPiece`] / [`ResolvedSlabPiece`]
//! value types (or a typed [`BattleSetupError`]) that the setup spawn loop then pours into
//! the ledgers / terrain entities / [`FloorCostGrid`](crate::terrain::floor::FloorCostGrid).

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    metric::CellLevel,
    pathfinder::MIN_MOVE_COST,
    situation::{BattleSetupError, Situation},
    slab::SlabHp,
    terrain::{
        entity::TerrainPieceKind,
        piece::{
            FootfallSound, TerrainGraphicKey, TerrainKindSpec, TerrainName, TerrainRegistry,
            TerrainSpec,
        },
    },
    tuning::MoveCost,
};

/// A pre-resolved cover piece — the structural stats and presentation hooks extracted
/// from a [`TerrainSpec`] for a cover (wall/cover/scatter) piece.
///
/// Produced by [`resolve_cover_spec`] from a
/// [`StructuralSpec`](crate::terrain::piece::StructuralSpec) plus the graphic/footfall
/// keys. Passed directly into the cover-ledger seeding and terrain-entity spawn, so the
/// resolution is done once and the values passed forward (no repeated registry reads).
pub(super) struct ResolvedCoverPiece {
    /// The cover's full (max) structural HP.
    pub(super) max_hp:           CoverHp,
    /// The clearance band this cover occupies (LOW / MID / HIGH).
    pub(super) height_band:      HeightBand,
    /// The cover's damage-reduction stat (same armor model as a ganger).
    pub(super) armor_protection: ArmorProtection,
    /// The penetration this cover shrugs off (same armor model as a ganger).
    pub(super) armor_hardness:   ArmorHardness,
    /// The entity piece kind (Wall or Cover) derived from the spec variant — carried on
    /// the spawned entity so the presenter can distinguish wall vs. cover by ECS query.
    pub(super) piece_kind:       TerrainPieceKind,
    /// The presentation graphic role key — carried on the spawned entity.
    pub(super) graphic:          TerrainGraphicKey,
    /// The footfall sound key — carried on the spawned entity (stubbed; no audio yet).
    pub(super) footfall:         FootfallSound,
}

/// A pre-resolved slab piece — the structural stats and presentation hooks extracted
/// from a [`TerrainSpec`] for a slab piece.
///
/// Produced by [`resolve_slab_spec`] from a
/// [`SlabPieceSpec`](crate::terrain::piece::SlabPieceSpec) + the graphic/footfall keys.
/// Passed directly into the slab-ledger seeding and terrain-entity spawn.
pub(super) struct ResolvedSlabPiece {
    /// The slab's full (max) structural HP.
    pub(super) max_hp:           SlabHp,
    /// The slab's damage-reduction stat (same armor model as a ganger and cover).
    pub(super) armor_protection: ArmorProtection,
    /// The penetration the slab shrugs off (same armor model).
    pub(super) armor_hardness:   ArmorHardness,
    /// The presentation graphic role key — carried on the spawned entity.
    pub(super) graphic:          TerrainGraphicKey,
    /// The footfall sound key — carried on the spawned entity (stubbed; no audio yet).
    pub(super) footfall:         FootfallSound,
}

/// Resolve a [`TerrainSpec`] as a **cover** piece (wall / cover / scatter) — extract
/// the [`StructuralSpec`](crate::terrain::piece::StructuralSpec) payload and presentation
/// hooks.
///
/// Returns `None` if the spec is not a structural variant (e.g. a floor or slab spec
/// was authored in the wrong list), which is treated as an authoring error at the call
/// site (the piece is re-validated as the correct kind there).
pub(super) fn resolve_cover_spec(
    name: &TerrainName,
    spec: &TerrainSpec,
) -> Option<ResolvedCoverPiece> {
    let (s, piece_kind) = match &spec.kind {
        TerrainKindSpec::Wall(s) => (s, TerrainPieceKind::Wall),
        TerrainKindSpec::Cover(s) | TerrainKindSpec::Scatter(s) => (s, TerrainPieceKind::Cover),
        _ => {
            // A floor or slab piece was named in a cover list — wrong kind.
            bevy::log::error!(
                "terrain piece {:?} is not a Wall/Cover/Scatter spec (found {:?}); \
                 it cannot be used in walls/scatter — check the situation authoring",
                name,
                spec.kind,
            );
            return None;
        }
    };
    Some(ResolvedCoverPiece {
        max_hp: s.max_hp,
        height_band: s.height_band,
        armor_protection: s.armor_protection,
        armor_hardness: s.armor_hardness,
        piece_kind,
        graphic: spec.graphic.clone(),
        footfall: spec.footfall.clone(),
    })
}

/// Resolve a [`TerrainSpec`] as a **slab** piece — extract the
/// [`SlabPieceSpec`](crate::terrain::piece::SlabPieceSpec) payload and presentation hooks.
///
/// Returns `None` if the spec is not a `Slab` variant, treated as an authoring error.
pub(super) fn resolve_slab_spec(
    name: &TerrainName,
    spec: &TerrainSpec,
) -> Option<ResolvedSlabPiece> {
    let TerrainKindSpec::Slab(s) = &spec.kind else {
        bevy::log::error!(
            "terrain piece {:?} is not a Slab spec (found {:?}); \
             it cannot be used in slabs — check the situation authoring",
            name,
            spec.kind,
        );
        return None;
    };
    Some(ResolvedSlabPiece {
        max_hp:           s.max_hp,
        armor_protection: s.armor_protection,
        armor_hardness:   s.armor_hardness,
        graphic:          spec.graphic.clone(),
        footfall:         spec.footfall.clone(),
    })
}

/// Resolve the `default_floor` and `floors` overrides from the situation against the
/// registry, returning `(default_cost, Vec<(CellLevel, MoveCost)>)`.
///
/// Returns the `fallback_floor_cost` as the default when the registry is absent or the
/// `default_floor` key is empty (the `#[serde(default)]` sentinel — preserves
/// pre-GTW-396 behavior for un-migrated fixtures). Validates floor costs ≥
/// [`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST); returns
/// `Err(`[`FloorCostBelowMinimum`](BattleSetupError::FloorCostBelowMinimum)`)` on
/// violation.
pub(super) fn resolve_floor_costs(
    terrain: Option<&TerrainRegistry>,
    situation: &Situation,
    fallback_floor_cost: MoveCost,
) -> Result<(MoveCost, Vec<(CellLevel, MoveCost)>), BattleSetupError> {
    // The admissibility floor for A*: `MIN_MOVE_COST` is a typed `MoveCost` constant
    // (the SAME source of truth the heuristic reads — no `as` cast at either site). Any
    // floor piece whose cost derefs below it breaks the heuristic; the comparison is
    // over the inner magnitudes via `Deref`.
    let minimum_cost = MIN_MOVE_COST;

    // If no registry or empty sentinel: skip registry resolution, use fallback.
    let Some(reg) = terrain else {
        return Ok((fallback_floor_cost, Vec::new()));
    };
    if situation.default_floor.is_empty() {
        return Ok((fallback_floor_cost, Vec::new()));
    }

    // Resolve the default floor piece.
    let default_cost = resolve_floor_piece_cost(reg, &situation.default_floor)?;
    if *default_cost < *minimum_cost {
        return Err(BattleSetupError::FloorCostBelowMinimum {
            piece:   situation.default_floor.clone(),
            cost:    default_cost,
            minimum: minimum_cost,
        });
    }

    // Resolve per-cell floor overrides.
    let mut overrides = Vec::with_capacity(situation.floors.len());
    for floor_spawn in &situation.floors {
        let cost = resolve_floor_piece_cost(reg, &floor_spawn.piece)?;
        if *cost < *minimum_cost {
            return Err(BattleSetupError::FloorCostBelowMinimum {
                piece: floor_spawn.piece.clone(),
                cost,
                minimum: minimum_cost,
            });
        }
        overrides.push((floor_spawn.at, cost));
    }

    Ok((default_cost, overrides))
}

/// Resolve a single floor terrain piece key to its [`MoveCost`], returning
/// [`BattleSetupError::TerrainNotFound`] if the key is absent or the spec is not a
/// `Floor` variant.
pub(super) fn resolve_floor_piece_cost(
    registry: &TerrainRegistry,
    name: &TerrainName,
) -> Result<MoveCost, BattleSetupError> {
    let Some(spec) = registry.spec(name) else {
        return Err(BattleSetupError::TerrainNotFound {
            piece: name.clone(),
        });
    };
    if let TerrainKindSpec::Floor(f) = &spec.kind {
        Ok(f.move_cost)
    } else {
        bevy::log::error!(
            "terrain piece {:?} is not a Floor spec (found {:?}); \
             it cannot be used as a floor piece — check the situation authoring",
            name,
            spec.kind,
        );
        Err(BattleSetupError::TerrainNotFound {
            piece: name.clone(),
        })
    }
}

/// Look up a terrain piece key in the registry, returning `Err(TerrainNotFound)` if
/// absent or if `terrain` is `None`. Used for cover and slab pre-resolution (not floor
/// — floor uses its own path that handles the empty-sentinel fallback).
pub(super) fn resolve_terrain_or_err<'a>(
    terrain: Option<&'a TerrainRegistry>,
    name: &TerrainName,
) -> Result<&'a TerrainSpec, BattleSetupError> {
    let Some(reg) = terrain else {
        return Err(BattleSetupError::TerrainNotFound {
            piece: name.clone(),
        });
    };
    reg.spec(name)
        .ok_or_else(|| BattleSetupError::TerrainNotFound {
            piece: name.clone(),
        })
}
