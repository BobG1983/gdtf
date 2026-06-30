//! The **path-blocking + vision-occlusion derivation rules** — the pure functions that
//! decide, from a [`TerrainDef`], whether a spawned terrain entity carries the
//! [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) marker (GTW-501,
//! child 482a) and the [`BlocksVision`](crate::terrain::entity::BlocksVision) component
//! (GTW-502, child 482b of the tag-driven-terrain epic GTW-482).
//!
//! These are the consumers of the formerly-inert sim-owned [`TerrainTag`]s. They are the
//! single authority for the **zero-regression** rules (GTW-501 D2 / GTW-502), so the spawn
//! loop, the tests, and any future reader all agree on one definition.

use super::{TerrainDef, TerrainSimKind, TerrainTag};
use crate::cover::HeightBand;

/// Whether a [`TerrainDef`] derives **path-blocking** — the rule that decides whether a
/// spawned terrain entity gets the
/// [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) marker (GTW-501 C1 / D2).
///
/// The derivation is the UNION of an explicit tag and a per-kind default:
///
/// 1. **Explicit tag.** The def's [`tags`](TerrainDef::tags) contains
///    [`TerrainTag::BlocksPathfinding`] — an authored opt-IN that ADDS path-blocking to a
///    kind that would not block by default (e.g. a [`Slab`](TerrainSimKind::Slab) tagged
///    `BlocksPathfinding` — a raised lip or barricade slab that bars footfall).
/// 2. **Per-kind default** ([`sim_kind_blocks_path`]). [`Wall`](TerrainSimKind::Wall) and
///    [`Cover`](TerrainSimKind::Cover) block the path by default; [`Slab`](TerrainSimKind::Slab)
///    does not (a slab is a floor/roof z-boundary you walk ON, not THROUGH). This default is
///    what makes existing walls/cover keep blocking with NO content migration — the
///    GTW-501 zero-regression guarantee (D2).
///
/// A def is path-blocking iff EITHER holds. The two are deliberately a union (not a
/// gate): the explicit tag can only ADD blocking, never remove a kind-default block —
/// removing a `Wall`/`Cover`'s block is out of scope for this child (it would be an
/// explicit "passable wall" tag, not modelled here).
///
/// PURE: a read over the borrowed def — no world access, no RNG, no side effects.
#[must_use]
pub fn derives_path_blocking(def: &TerrainDef) -> bool {
    let explicit = def.tags.contains(&TerrainTag::BlocksPathfinding);
    explicit || sim_kind_blocks_path(&def.sim_kind)
}

/// Whether a [`TerrainSimKind`] blocks the path **by default** — `true` for
/// [`Wall`](TerrainSimKind::Wall) and [`Cover`](TerrainSimKind::Cover), `false` for
/// [`Slab`](TerrainSimKind::Slab) (GTW-501 D2).
///
/// The per-kind half of [`derives_path_blocking`]. A wall fills the cell and standing
/// cover obstructs it, so both bar a path step by default — the same kinds the kind-based
/// [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked) treats as
/// blocking today, so deriving the marker from this default reproduces the existing pathing
/// exactly (zero regression). A slab is a horizontal z-boundary you stand on, not a
/// same-storey obstruction, so it does not block by default — only an explicit
/// [`BlocksPathfinding`](TerrainTag::BlocksPathfinding) tag makes one block (the C1
/// opt-in).
#[must_use]
pub const fn sim_kind_blocks_path(sim_kind: &TerrainSimKind) -> bool {
    matches!(
        sim_kind,
        TerrainSimKind::Wall { .. } | TerrainSimKind::Cover { .. }
    )
}

/// Whether a [`TerrainDef`] derives **vision occlusion**, and at WHICH [`HeightBand`] it
/// occludes — the rule that decides whether a spawned terrain entity gets the
/// [`BlocksVision`](crate::terrain::entity::BlocksVision) component and what band it carries
/// (GTW-502 C1, the LoS/FoV mirror of [`derives_path_blocking`]).
///
/// Vision occlusion is **height-aware** (per the user-pinned GTW-482 refinement): an
/// occluder does not block sight uniformly — it occludes the band it physically fills, so a
/// LOW occluder blocks a LOW sightline but a round/eye-line one band HIGHER sails over it,
/// EXACTLY as a [`CoverEntry`](crate::cover::CoverEntry)'s band already gates the march
/// (`docs/combat/resolution.md` §3). The function therefore returns the OCCLUDING BAND, not
/// a bare `bool`:
///
/// 1. **Explicit [`BlocksVision`](TerrainTag::BlocksVision) tag.** The def's
///    [`tags`](TerrainDef::tags) contains [`TerrainTag::BlocksVision`] — an authored opt-IN
///    that ADDS vision occlusion to a kind that would not occlude by default (e.g. a
///    [`Slab`](TerrainSimKind::Slab) tagged `BlocksVision` — an opaque screen / blast wall a
///    slab def authors). The occluding band is the `sim_kind`'s own
///    [`height_band`](TerrainSimKind) when it has one (`Wall`/`Cover`), else
///    [`HeightBand::High`] for a tagged `Slab` (a slab has no band; a deliberately-opaque
///    slab fills the storey, so it occludes the tallest band — nothing within the storey
///    sees over it).
/// 2. **Per-kind default** ([`sim_kind_occludes_vision`]). [`Wall`](TerrainSimKind::Wall)
///    and [`Cover`](TerrainSimKind::Cover) occlude vision by default at their authored
///    `height_band`; [`Slab`](TerrainSimKind::Slab) does NOT (a slab is a horizontal
///    z-boundary the SLAB march already stops sight at via the `SurfaceGrid` — it is not a
///    same-storey occluder, so it gets no `BlocksVision` band unless explicitly tagged).
///
/// **Zero-regression reconciliation (GTW-502 D-CRITICAL).** A `Wall`/`Cover` is ALREADY
/// seeded into the [`CoverLedger`](crate::cover::CoverLedger) at setup carrying this same
/// `height_band`, and [`impact_at`](crate::march) ALREADY occludes vision on it via that
/// ledger entry — so walls/cover occlude sight TODAY, height-aware. Deriving the SAME band
/// here means the new tag-derived occluder reproduces a Wall/Cover's existing occlusion
/// EXACTLY (same band gate, same destroyed-cover exclusion) — an idempotent re-block that
/// never changes their behaviour and never double-counts (the cover-ledger clause fires
/// first in `impact_at` for an intact `Wall`/`Cover`, and the new clause uses an identical
/// band test, so it can only agree). The NET-NEW behaviour is the explicit-tag opt-in for a
/// `Slab`, which the cover ledger never held — a desirable gap-closer, not a regression.
///
/// Returns `None` when the def derives NO vision occlusion (an untagged `Slab`).
///
/// PURE: a read over the borrowed def — no world access, no RNG, no side effects.
#[must_use]
pub fn derives_vision_occlusion(def: &TerrainDef) -> Option<HeightBand> {
    if def.tags.contains(&TerrainTag::BlocksVision) {
        // The explicit opt-in: occlude at the sim_kind's own band, or HIGH for a tagged Slab
        // (a slab has no band; an opaque slab fills the storey, so it occludes the tallest).
        return Some(sim_kind_band(&def.sim_kind).unwrap_or(HeightBand::High));
    }
    // The per-kind default: a Wall/Cover occludes at its authored band, a Slab does not.
    sim_kind_band_when_occludes(&def.sim_kind)
}

/// Whether a [`TerrainSimKind`] occludes vision **by default** — `true` for
/// [`Wall`](TerrainSimKind::Wall) and [`Cover`](TerrainSimKind::Cover), `false` for
/// [`Slab`](TerrainSimKind::Slab) (GTW-502 C1).
///
/// The per-kind half of [`derives_vision_occlusion`], exposed as a `bool` predicate (the
/// [`sim_kind_blocks_path`] mirror) for readers/tests that want the kind-default flag without
/// the band. A wall fills the cell and standing cover obstructs it, so both occlude a
/// same-storey sightline by default — the same kinds already seeded into the
/// [`CoverLedger`](crate::cover::CoverLedger) (so deriving the occluder from this default
/// reproduces the existing `LoS` exactly, zero regression). A slab is a horizontal z-boundary
/// the slab march already handles, not a same-storey occluder, so it does not occlude by
/// default — only an explicit [`BlocksVision`](TerrainTag::BlocksVision) tag makes one.
#[must_use]
pub const fn sim_kind_occludes_vision(sim_kind: &TerrainSimKind) -> bool {
    matches!(
        sim_kind,
        TerrainSimKind::Wall { .. } | TerrainSimKind::Cover { .. }
    )
}

/// The occluding [`HeightBand`] of a `Wall`/`Cover` `sim_kind`, or `None` for a `Slab` — the
/// per-kind-default arm of [`derives_vision_occlusion`].
const fn sim_kind_band_when_occludes(sim_kind: &TerrainSimKind) -> Option<HeightBand> {
    match sim_kind {
        TerrainSimKind::Wall { height_band, .. } | TerrainSimKind::Cover { height_band, .. } => {
            Some(*height_band)
        }
        TerrainSimKind::Slab { .. } => None,
    }
}

/// The authored [`HeightBand`] a `sim_kind` carries, or `None` for a [`Slab`] (which has no
/// band — it spans the whole z-boundary, `docs/combat/resolution.md` §2). The explicit-tag
/// arm of [`derives_vision_occlusion`] reads this to band a tagged `Wall`/`Cover` at its own
/// band and falls back to [`HeightBand::High`] for a tagged `Slab`.
const fn sim_kind_band(sim_kind: &TerrainSimKind) -> Option<HeightBand> {
    sim_kind_band_when_occludes(sim_kind)
}

/// Whether a [`TerrainDef`] is **openable** — carries the
/// [`TerrainTag::Openable`] tag (GTW-503 C1, child 482c of the tag-driven-terrain epic
/// GTW-482).
///
/// The single authority for "is this a door / hatch?" — read by the setup spawn loop to
/// decide whether to attach the [`OpenState`](crate::terrain::openable::OpenState) component
/// (default [`Closed`](crate::terrain::openable::OpenState::Closed)) and force the closed
/// blocking pair (C2). An openable piece's blocking lifecycle is OWNED by GTW-503 (the toggle
/// drives the [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) +
/// [`BlocksVision`](crate::terrain::entity::BlocksVision) add/remove), NOT by the
/// kind-default [`derives_path_blocking`] / [`derives_vision_occlusion`] derivation — which
/// continues to govern every NON-openable piece unchanged.
///
/// PURE: a read over the borrowed def — no world access, no RNG, no side effects.
#[must_use]
pub fn is_openable(def: &TerrainDef) -> bool {
    def.tags.contains(&TerrainTag::Openable)
}

/// The [`HeightBand`] a **closed openable** piece occludes vision at (GTW-503 C2).
///
/// A CLOSED door blocks vision EVEN IF its `sim_kind` would not occlude by default (e.g. an
/// openable [`Slab`](TerrainSimKind::Slab) hatch). So this is NOT
/// [`derives_vision_occlusion`] (which returns `None` for an untagged slab): a closed
/// openable ALWAYS occludes, at the `sim_kind`'s own band when it has one
/// ([`Wall`](TerrainSimKind::Wall) / [`Cover`](TerrainSimKind::Cover)), else
/// [`HeightBand::High`] (a slab spans the whole storey, so a closed slab hatch occludes the
/// tallest band — nothing within the storey sees over it, mirroring the tagged-slab arm of
/// [`derives_vision_occlusion`]).
///
/// PURE: a read over the borrowed def — no world access, no RNG, no side effects.
#[must_use]
pub fn closed_openable_vision_band(def: &TerrainDef) -> HeightBand {
    sim_kind_band(&def.sim_kind).unwrap_or(HeightBand::High)
}
