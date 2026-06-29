//! The terrain-resolution cluster — the pure helpers
//! [`setup_battle`](super::setup_battle) calls to turn each authored terrain DEFINITION
//! KEY (cover / slab) into a resolved spec BEFORE any entity is spawned (abort-first),
//! kept out of `setup.rs` so the spawn orchestration stays single-responsibility.
//!
//! GTW-491 migration (child T07a of the GTW-476 data-model refactor): each authored
//! [`TerrainUuid`] is resolved against the
//! [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry). The resolved
//! definition's [`TerrainSimKind`] supplies the structural stats + entity
//! [`TerrainPieceKind`], and its [`TerrainPresenterKind`] supplies the presentation hooks
//! ([`TerrainGraphicKey`] for ALL kinds incl. `Wall`, an optional [`FootfallSound`] for
//! `Slab` only).
//!
//! Every helper here is registry-read + validation only — no `Commands`, no spawn, no
//! world mutation. They return the resolved [`ResolvedCoverPiece`] / [`ResolvedSlabPiece`]
//! value types (or a typed [`BattleSetupError`]) that the setup spawn loop then pours into
//! the ledgers / terrain entities.

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    situation::BattleSetupError,
    slab::SlabHp,
    terrain::{
        def::{TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainSimKind, TerrainUuid},
        entity::TerrainPieceKind,
        piece::{FootfallSound, TerrainGraphicKey},
    },
};

/// A pre-resolved cover piece — the structural stats and presentation hooks extracted
/// from a [`TerrainDef`] for a cover (wall / cover) piece.
///
/// Produced by [`resolve_cover_def`] from a [`TerrainSimKind`] `Wall`/`Cover` variant plus
/// the presenter graphic key. Passed directly into the cover-ledger seeding and
/// terrain-entity spawn, so the resolution is done once and the values passed forward (no
/// repeated registry reads).
pub(super) struct ResolvedCoverPiece {
    /// The cover's full (max) structural HP.
    pub(super) max_hp:           CoverHp,
    /// The clearance band this cover occupies (LOW / MID / HIGH).
    pub(super) height_band:      HeightBand,
    /// The cover's damage-reduction stat (same armor model as a ganger).
    pub(super) armor_protection: ArmorProtection,
    /// The penetration this cover shrugs off (same armor model as a ganger).
    pub(super) armor_hardness:   ArmorHardness,
    /// The entity piece kind (Wall or Cover) derived from the def's [`TerrainSimKind`]
    /// variant — carried on the spawned entity so the presenter can distinguish wall vs.
    /// cover by ECS query AND so the occupancy [`TerrainKind`](crate::occupancy::TerrainKind)
    /// is derived from the DEF VARIANT, never from authoring-list membership (the GTW-483 /
    /// T01 invariant the GTW-491 rewrite preserves).
    pub(super) piece_kind:       TerrainPieceKind,
    /// The presentation graphic role key — carried on the spawned entity (NET-NEW for `Wall`
    /// in the GTW-491 model: a wall entity now carries a graphic, which it did not on the old
    /// model; GTW-493 reads it).
    pub(super) graphic:          TerrainGraphicKey,
}

/// A pre-resolved slab piece — the structural stats and presentation hooks extracted
/// from a [`TerrainDef`] for a slab piece.
///
/// Produced by [`resolve_slab_def`] from a [`TerrainSimKind::Slab`] variant + the presenter
/// graphic key + the OPTIONAL slab footfall. Passed directly into the slab-ledger seeding
/// and terrain-entity spawn.
pub(super) struct ResolvedSlabPiece {
    /// The slab's full (max) structural HP.
    pub(super) max_hp:           SlabHp,
    /// The slab's damage-reduction stat (same armor model as a ganger and cover).
    pub(super) armor_protection: ArmorProtection,
    /// The penetration the slab shrugs off (same armor model).
    pub(super) armor_hardness:   ArmorHardness,
    /// The presentation graphic role key — carried on the spawned entity.
    pub(super) graphic:          TerrainGraphicKey,
    /// The OPTIONAL footfall sound key — carried on the spawned entity when the def names one
    /// (slab-only in the GTW-491 model; `None` when the slab def authors no footfall).
    pub(super) footfall:         Option<FootfallSound>,
}

/// Resolve a [`TerrainDef`] as a **cover** piece (wall / cover) — extract the
/// [`TerrainSimKind`] `Wall`/`Cover` structural stats + the [`TerrainPieceKind`] + the
/// presenter graphic.
///
/// Returns `None` if the def is not a `Wall`/`Cover` sim-kind (e.g. a slab def was authored
/// in a cover list), which is treated as an authoring error at the call site (the piece is
/// re-validated as the correct kind there).
pub(super) fn resolve_cover_def(key: &TerrainUuid, def: &TerrainDef) -> Option<ResolvedCoverPiece> {
    let (max_hp, armor_protection, armor_hardness, height_band, piece_kind) = match &def.sim_kind {
        TerrainSimKind::Wall {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            *hp,
            *armor_protection,
            *armor_hardness,
            *height_band,
            TerrainPieceKind::Wall,
        ),
        TerrainSimKind::Cover {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            *hp,
            *armor_protection,
            *armor_hardness,
            *height_band,
            TerrainPieceKind::Cover,
        ),
        TerrainSimKind::Slab { .. } => {
            // A slab def was named in a cover list — wrong kind.
            bevy::log::error!(
                "terrain def {:?} is not a Wall/Cover sim-kind (found {:?}); \
                 it cannot be used in walls/scatter — check the situation authoring",
                key,
                def.sim_kind,
            );
            return None;
        }
    };
    Some(ResolvedCoverPiece {
        max_hp,
        height_band,
        armor_protection,
        armor_hardness,
        piece_kind,
        // NET-NEW (GTW-491): the graphic comes from the def's presenter_kind for ALL kinds,
        // INCLUDING Wall — a wall entity now carries a TerrainGraphicKey (the fact GTW-493
        // reads). A presenter-kind variant that disagrees with the sim-kind is an authoring
        // error; fall through to the def's own graphic regardless of which presenter variant
        // it is (every variant carries `graphic_name`).
        graphic: presenter_graphic(&def.presenter_kind),
    })
}

/// Resolve a [`TerrainDef`] as a **slab** piece — extract the [`TerrainSimKind::Slab`]
/// structural stats + the presenter graphic + the OPTIONAL footfall.
///
/// Returns `None` if the def is not a `Slab` sim-kind, treated as an authoring error.
pub(super) fn resolve_slab_def(key: &TerrainUuid, def: &TerrainDef) -> Option<ResolvedSlabPiece> {
    let TerrainSimKind::Slab {
        hp,
        armor_protection,
        armor_hardness,
    } = &def.sim_kind
    else {
        bevy::log::error!(
            "terrain def {:?} is not a Slab sim-kind (found {:?}); \
             it cannot be used in slabs — check the situation authoring",
            key,
            def.sim_kind,
        );
        return None;
    };
    Some(ResolvedSlabPiece {
        max_hp:           *hp,
        armor_protection: *armor_protection,
        armor_hardness:   *armor_hardness,
        graphic:          presenter_graphic(&def.presenter_kind),
        // Footfall is Slab-only in the GTW-491 model — the def's presenter Slab variant carries
        // an Option; a non-Slab presenter variant authored on a Slab sim-kind is an authoring
        // mismatch and yields None (no footfall).
        footfall:         match &def.presenter_kind {
            TerrainPresenterKind::Slab { footfall, .. } => footfall.clone(),
            TerrainPresenterKind::Wall { .. } | TerrainPresenterKind::Cover { .. } => None,
        },
    })
}

/// The graphic role key off a [`TerrainPresenterKind`] — every variant carries one
/// (`graphic_name`), so the GTW-491 net-new "graphic for ALL kinds including Wall" fact holds
/// by construction.
fn presenter_graphic(presenter_kind: &TerrainPresenterKind) -> TerrainGraphicKey {
    match presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name.clone(),
    }
}

/// Look up a terrain definition KEY in the registry, returning `Err(TerrainNotFound)` if
/// absent or if `terrain` is `None`. Used for cover and slab pre-resolution.
pub(super) fn resolve_terrain_or_err<'a>(
    terrain: Option<&'a TerrainDefRegistry>,
    key: &TerrainUuid,
) -> Result<&'a TerrainDef, BattleSetupError> {
    let Some(reg) = terrain else {
        return Err(BattleSetupError::TerrainNotFound { piece: *key });
    };
    reg.def(key)
        .ok_or(BattleSetupError::TerrainNotFound { piece: *key })
}
