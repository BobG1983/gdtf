//! The **path-blocking + vision-occlusion derivation rules** — the pure functions that
//! decide, from a [`TerrainDef`], whether a spawned terrain entity carries the
//! [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) marker (GTW-501,
//! child 482a) and the [`BlocksVision`](crate::terrain::entity::BlocksVision) component
//! (GTW-502, child 482b of the tag-driven-terrain epic GTW-482).
//!
//! These are the consumers of the formerly-inert sim-owned [`TerrainTag`]s. They are the
//! single authority for the **zero-regression** rules (GTW-501 D2 / GTW-502), so the spawn
//! loop, the tests, and any future reader all agree on one definition.

use bevy::prelude::Deref;

use super::{LosBlocking, TerrainDef, TerrainSimKind, TerrainTag};
use crate::{
    cover::HeightBand,
    occupancy::{OccludesVision, PathBlocked},
    terrain::entity::TerrainPieceKind,
};

/// Whether a [`TerrainDef`] is **openable** — a door / hatch that carries the
/// [`TerrainTag::Openable`] tag (the answer [`is_openable`] returns).
///
/// A named newtype over `bool` (no-bare-types: door-ness is a domain fact, not a bare
/// boolean). `true` gates the spawn loop's [`OpenState`](crate::terrain::openable::OpenState)
/// attachment. Private inner + derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Openable(bool);

impl Openable {
    /// Build an openable answer from its boolean state.
    #[must_use]
    pub const fn new(openable: bool) -> Self {
        Self(openable)
    }
}

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
/// gate): the explicit tag can only ADD blocking, never remove a kind-default block.
///
/// **Authored override (GTW-587).** An explicit [`blocks_pathing`](TerrainDef::blocks_pathing)
/// `Some(bool)` WINS OUTRIGHT over both the tag and the kind default — `Some(true)` forces the
/// def to block (e.g. a `Slab` railing), `Some(false)` forces it walkable (e.g. a decorative
/// wall you can walk through), letting path-blocking vary per-def WITHOUT a new sim kind. The
/// field defaults to `None` (`#[serde(default)]`), which falls through to the tag-∪-kind rule
/// below — so every shipped def (none author it) derives EXACTLY as before (zero regression).
///
/// PURE: a read over the borrowed def — no world access, no RNG, no side effects.
#[must_use]
pub fn derives_path_blocking(def: &TerrainDef) -> PathBlocked {
    // GTW-587: an explicit per-def override wins over the tag ∪ kind-default rule.
    if let Some(over) = def.blocks_pathing {
        return PathBlocked::new(*over);
    }
    let explicit = def.tags.contains(&TerrainTag::BlocksPathfinding);
    PathBlocked::new(explicit || *sim_kind_blocks_path(&def.sim_kind))
}

/// Whether a [`TerrainSimKind`] blocks the path **by default** — `true` for
/// [`Wall`](TerrainSimKind::Wall), [`Cover`](TerrainSimKind::Cover), and
/// [`Emplacement`](TerrainSimKind::Emplacement) (a cover-like smashable structure that
/// fills its cell); `false` for [`Slab`](TerrainSimKind::Slab) (GTW-501 D2 / GTW-543).
///
/// The per-kind half of [`derives_path_blocking`]. A wall fills the cell and standing
/// cover obstructs it, so both bar a path step by default — the same kinds the kind-based
/// [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked) treats as
/// blocking today, so deriving the marker from this default reproduces the existing pathing
/// exactly (zero regression). A slab is a horizontal z-boundary you stand on, not a
/// same-storey obstruction, so it does not block by default — only an explicit
/// [`BlocksPathfinding`](TerrainTag::BlocksPathfinding) tag makes one block (the C1
/// opt-in).
///
/// A kind-IDENTITY decision (no per-variant payload), so it reads the canonical
/// [`TerrainPieceKind`] projection ([`TerrainSimKind::kind`] — GTW-574 C2).
#[must_use]
pub const fn sim_kind_blocks_path(sim_kind: &TerrainSimKind) -> PathBlocked {
    PathBlocked::new(matches!(
        sim_kind.kind(),
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement
    ))
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
/// 2. **Per-kind default** ([`sim_kind_default_los`]). [`Wall`](TerrainSimKind::Wall)
///    occludes vision FULLY by default ([`LosBlocking::Full`] → the whole storey; a wall fills
///    the cell, nothing within the storey sees over it), while [`Cover`](TerrainSimKind::Cover)
///    and [`Emplacement`](TerrainSimKind::Emplacement) occlude only UP TO their authored
///    `height_band` ([`LosBlocking::UpToHeightBand`] — a higher sightline clears them);
///    [`Slab`](TerrainSimKind::Slab) does NOT occlude (a slab is a horizontal z-boundary the
///    SLAB march already stops sight at via the `SurfaceGrid` — it is not a same-storey
///    occluder, so it gets no `BlocksVision` band unless explicitly tagged).
///
/// **Zero-regression reconciliation (GTW-502 D-CRITICAL, refined GTW-587).** A
/// `Wall`/`Cover`/`Emplacement` is ALREADY seeded into the
/// [`CoverLedger`](crate::cover::CoverLedger) at setup carrying its `height_band`, and
/// [`impact_at`](crate::march) ALREADY occludes vision on it via that ledger entry — so
/// walls/cover occlude sight TODAY, height-aware. A `Cover`/`Emplacement`'s kind default
/// ([`UpToHeightBand`](LosBlocking::UpToHeightBand)) derives that SAME band, reproducing its
/// existing occlusion EXACTLY. A `Wall`'s kind default is [`Full`](LosBlocking::Full) →
/// [`HeightBand::High`] (the GTW-587 model, "walls occlude fully"); every SHIPPED wall def
/// authors `height_band: High`, so its derived High band EQUALS its ledger band and shipped
/// behaviour is identical (AC1). The derivation and the ledger use an identical band
/// test, so on an intact piece they can only agree (the cover-ledger clause fires first in
/// `impact_at`, same destroyed-cover exclusion). The NET-NEW behaviours are the explicit-tag
/// opt-in for a `Slab` (which the ledger never held) and — for a hypothetical untagged
/// NON-`High` wall (none shipped) — the deliberate whole-storey `Full` occlusion the GTW-587
/// kind default mandates in place of the pre-587 own-band derivation.
///
/// **Authored override (GTW-587).** An explicit
/// [`blocks_los`](TerrainDef::blocks_los) `Some(LosBlocking)` WINS OUTRIGHT over both the
/// `BlocksVision` tag and the kind default, letting LoS-occlusion vary per-def WITHOUT a new
/// sim kind: `Some(LosBlocking::None)` makes a piece LoS-transparent, `Some(Full)` occludes the
/// whole storey, `Some(UpToHeightBand)` occludes only up to the def's own band. The field
/// defaults to `None` (`#[serde(default)]`), which falls through to the tag-then-kind default —
/// so every shipped def derives EXACTLY the same band as before (zero regression).
///
/// Returns `None` when the def derives NO vision occlusion (an untagged `Slab`, or an explicit
/// `LosBlocking::None`).
///
/// PURE: a read over the borrowed def — no world access, no RNG, no side effects.
#[must_use]
pub fn derives_vision_occlusion(def: &TerrainDef) -> Option<HeightBand> {
    los_blocking_to_band(resolved_los_blocking(def), &def.sim_kind)
}

/// Resolve a [`TerrainDef`]'s effective [`LosBlocking`] mode (GTW-587) — the authored
/// [`blocks_los`](TerrainDef::blocks_los) override if present, else the tag-then-kind default.
///
/// The precedence, highest first:
///
/// 1. **Authored override.** An explicit [`blocks_los`](TerrainDef::blocks_los) `Some(mode)`
///    wins outright.
/// 2. **Explicit [`BlocksVision`](TerrainTag::BlocksVision) tag.** The additive opt-in occludes
///    at the def's own band — [`UpToHeightBand`](LosBlocking::UpToHeightBand) for a kind WITH a
///    band (`Wall`/`Cover`/`Emplacement`), [`Full`](LosBlocking::Full) for a band-less `Slab`
///    (an opaque slab fills the storey). This reproduces the pre-GTW-587 tagged-slab-at-HIGH /
///    tagged-wall-at-its-band behaviour EXACTLY once mapped back through
///    [`los_blocking_to_band`].
/// 3. **Per-kind default** ([`sim_kind_default_los`]): `Wall` → `Full`, `Cover`/`Emplacement` →
///    `UpToHeightBand`, `Slab` → `None`.
///
/// PURE: a read over the borrowed def.
#[must_use]
pub fn resolved_los_blocking(def: &TerrainDef) -> LosBlocking {
    if let Some(over) = def.blocks_los {
        return over;
    }
    if def.tags.contains(&TerrainTag::BlocksVision) {
        // The additive tag occludes at the def's OWN band: UpToHeightBand for a banded kind, or
        // Full for a band-less Slab (which fills the storey) — mapped back through
        // `los_blocking_to_band` this yields Some(band) / Some(High), the pre-GTW-587 shape.
        return match sim_kind_band(&def.sim_kind) {
            Some(_) => LosBlocking::UpToHeightBand,
            None => LosBlocking::Full,
        };
    }
    sim_kind_default_los(&def.sim_kind)
}

/// The KIND-DERIVED default [`LosBlocking`] mode for a [`TerrainSimKind`] (GTW-587) — the
/// per-kind arm of [`resolved_los_blocking`].
///
/// [`Wall`](TerrainSimKind::Wall) → [`Full`](LosBlocking::Full) (a wall fills the cell; nothing
/// within the storey sees over it); [`Cover`](TerrainSimKind::Cover) /
/// [`Emplacement`](TerrainSimKind::Emplacement) → [`UpToHeightBand`](LosBlocking::UpToHeightBand)
/// (chest-high cover a higher sightline clears, at the def's own band);
/// [`Slab`](TerrainSimKind::Slab) → [`None`](LosBlocking::None) (a z-boundary the slab march
/// already stops sight at, not a same-storey occluder).
///
/// A kind-IDENTITY decision, so it reads the canonical [`TerrainPieceKind`] projection.
#[must_use]
pub const fn sim_kind_default_los(sim_kind: &TerrainSimKind) -> LosBlocking {
    match sim_kind.kind() {
        TerrainPieceKind::Wall => LosBlocking::Full,
        TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => LosBlocking::UpToHeightBand,
        TerrainPieceKind::Slab => LosBlocking::None,
    }
}

/// Map a resolved [`LosBlocking`] mode to the concrete occluding [`HeightBand`] the sim's
/// `VisionBlocking` surface reads (GTW-587), given the def's `sim_kind` for the band-relative
/// mode:
///
/// - [`Full`](LosBlocking::Full) → `Some(`[`HeightBand::High`]`)` — occludes the tallest band,
///   so nothing within the storey clears it.
/// - [`UpToHeightBand`](LosBlocking::UpToHeightBand) → `Some(`the def's own band`)`, or
///   [`HeightBand::High`] for a band-less [`Slab`](TerrainSimKind::Slab) (it spans the storey).
/// - [`None`](LosBlocking::None) → `None` — no occluder (LoS-transparent).
#[must_use]
pub const fn los_blocking_to_band(
    los: LosBlocking,
    sim_kind: &TerrainSimKind,
) -> Option<HeightBand> {
    match los {
        LosBlocking::Full => Some(HeightBand::High),
        LosBlocking::UpToHeightBand => match sim_kind_band(sim_kind) {
            Some(band) => Some(band),
            None => Some(HeightBand::High),
        },
        LosBlocking::None => None,
    }
}

/// Whether a [`TerrainSimKind`] occludes vision **by default** — `true` for
/// [`Wall`](TerrainSimKind::Wall), [`Cover`](TerrainSimKind::Cover), and
/// [`Emplacement`](TerrainSimKind::Emplacement); `false` for [`Slab`](TerrainSimKind::Slab)
/// (GTW-502 C1 / GTW-543).
///
/// A `bool` PRESENCE predicate — whether the kind derives ANY vision occlusion, NOT at which
/// band or in which mode. The per-kind mode/band arm is [`sim_kind_default_los`] (`Wall` →
/// `Full`, `Cover`/`Emplacement` → `UpToHeightBand`, `Slab` → `None`); this predicate is `true`
/// exactly when that default is non-[`None`](LosBlocking::None). It is exposed as the
/// [`sim_kind_blocks_path`] mirror for readers/tests that want the kind-default flag without
/// resolving the [`LosBlocking`] mode. A wall fills the cell and standing cover obstructs it,
/// so both occlude a same-storey sightline by default; a slab is a horizontal z-boundary the
/// slab march already handles, not a same-storey occluder, so it does not occlude by default —
/// only an explicit [`BlocksVision`](TerrainTag::BlocksVision) tag makes one.
///
/// A kind-IDENTITY decision (no per-variant payload), so it reads the canonical
/// [`TerrainPieceKind`] projection ([`TerrainSimKind::kind`] — GTW-574 C2).
#[must_use]
pub const fn sim_kind_occludes_vision(sim_kind: &TerrainSimKind) -> OccludesVision {
    OccludesVision::new(matches!(
        sim_kind.kind(),
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement
    ))
}

/// The authored [`HeightBand`] a `Wall`/`Cover`/`Emplacement` `sim_kind` carries, or `None` for
/// a [`Slab`] (which has no band — it spans the whole z-boundary, `docs/combat/resolution.md`
/// §2). The band ACCESSOR read by [`resolved_los_blocking`] (to band a `BlocksVision`-tagged
/// piece at its own band), by [`los_blocking_to_band`] (to resolve the
/// [`UpToHeightBand`](LosBlocking::UpToHeightBand) mode to a concrete band), and by
/// [`closed_openable_vision_band`] (falling back to [`HeightBand::High`] for a band-less slab).
const fn sim_kind_band(sim_kind: &TerrainSimKind) -> Option<HeightBand> {
    match sim_kind {
        TerrainSimKind::Wall { height_band, .. }
        | TerrainSimKind::Cover { height_band, .. }
        | TerrainSimKind::Emplacement { height_band, .. } => Some(*height_band),
        TerrainSimKind::Slab { .. } => None,
    }
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
pub fn is_openable(def: &TerrainDef) -> Openable {
    Openable::new(def.tags.contains(&TerrainTag::Openable))
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
