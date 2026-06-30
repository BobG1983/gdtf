//! The **path-blocking derivation rule** — the pure function that decides, from a
//! [`TerrainDef`], whether a spawned terrain entity carries the
//! [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) marker (GTW-501,
//! child 482a of the tag-driven-terrain epic GTW-482).
//!
//! This is the FIRST consumer of the formerly-inert sim-owned [`TerrainTag`]s. It is the
//! single authority for the GTW-501 D2 **zero-regression** rule, so the spawn loop, the
//! tests, and any future reader all agree on one definition.

use super::{TerrainDef, TerrainSimKind, TerrainTag};

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
