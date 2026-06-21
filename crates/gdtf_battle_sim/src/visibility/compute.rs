//! The pure value-transform helpers behind the squad fog — [`union_fov`] (the
//! squad VISIBLE union over a bounded candidate set) and [`accrue`] (the monotone
//! VISIBLE-replaces / EXPLORED-grows fold). NO I/O, no `Commands`, no `&mut World`:
//! these are the value transforms GTW-341's recompute system calls (GTW-340, leaf 4).

use bevy::platform::collections::HashSet;

use crate::{
    cover::CoverLedger,
    ganger::{Facing, LifeState, Position, Stance, StanceKind},
    los::{Observer, Target, can_see},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::CombatTuning,
    visibility::SquadVisibility,
};

/// One conscious **player-faction observer** the squad-FOV union sweeps from — a
/// borrow-view over the ganger components [`union_fov`] needs to anchor each candidate
/// sight probe (GTW-340 clause 5).
///
/// Mirrors the [`Observer`] borrow-view house style: refs assembled from the live
/// ganger components, not a stored struct. Only **conscious** (`life.is_active()`)
/// observers contribute — `union_fov` filters on [`life`](FovObserver::life) so a
/// Downed / Dead ganger (and, by the caller's own filtering, any non-player ganger)
/// reveals nothing. Carries the eye anchor inputs ([`position`](FovObserver::position) +
/// [`stance`](FovObserver::stance) + [`facing`](FovObserver::facing)) [`can_see`] reads
/// per candidate.
#[derive(Debug, Clone, Copy)]
pub struct FovObserver<'a> {
    /// The observer's `(cell, level)` grid position — the disc center + eye datum.
    pub position: &'a Position,
    /// The observer's stance — selects the per-stance muzzle level-fraction the eye
    /// sits at (the same anchor [`can_see`] / [`has_los`](crate::los::has_los) use).
    pub stance:   &'a Stance,
    /// The observer's facing — carried to build the [`Observer`] view; the
    /// facing-neutral eye does not read it (FOV is omni-directional).
    pub facing:   &'a Facing,
    /// The observer's life state — only [`LifeState::is_active`] (Alive) contributes;
    /// an inactive observer is skipped.
    pub life:     LifeState,
}

/// The squad **VISIBLE** union over a **bounded candidate set** — the cells some
/// conscious player-faction `observers` member currently sees (GTW-340 clause 5).
///
/// Candidate set (concrete, bounded — clause 5 / 6): for each conscious observer, the
/// candidates are the grid's **authored / occupied** `(cell, level)` entries
/// ([`OccupancyGrid::authored_or_occupied_cells`]) that fall **within that observer's
/// Chebyshev disc** (`max(|dx|, |dy|) <= *view_range`, across the level axis) — NOT
/// every cell in the `~29 × 29 × 8` disc. The rendered layer IS the fog mask
/// (`docs/combat/visibility.md`): empty air has no cell to reveal, and a far
/// out-of-disc cell is never tested. This is the same disc bound [`can_see`] applies
/// internally — the explicit filter here is what keeps the candidate scan
/// **disc-bounded over the sparse authored content**, never a blind `60 × 60 × 8`
/// (or full-disc-air) scan.
///
/// For each candidate, the occupant's band is resolved the EXACT way the shot pipeline
/// resolves it — `cover.peek(at).map(height_band).or_else(|| occupancy.occupant_band(at))`
/// (the [`TargetGeometry::compose`](crate::fire) pattern) — and the [`Target`] view is
/// built at that band (an inert [`StanceKind::Standing`] fallback for the no-band case,
/// mirroring the shot pipeline) before [`can_see`] runs. So the union's enemy-visible
/// decision uses the **same banded anchor the shot pipeline aims at**: a crouched
/// (non-Standing) occupant is classified by its occupant-band silhouette, not a generic
/// cell center. The [`can_see`]-true candidates fold into one VISIBLE set.
///
/// Read-only over the grids; render-free, RNG-free, deterministic, no `&mut World` /
/// `Commands`. The `is_dead` corpse predicate is the same read-only pass-through
/// [`can_see`] / [`has_los`](crate::los::has_los) take.
///
/// PERF (clause 6): the recompute (GTW-341) rides EVERY accepted `Changed<Position>`
/// move step — the per-step ambush-reveal trigger — so this candidate scan MUST stay
/// the disc-bounded authored/occupied set above, never a blind `60 × 60 × 8` (or
/// full-disc air) scan per observer per step.
#[must_use]
pub fn union_fov(
    observers: &[FovObserver],
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: impl Fn(bevy::prelude::Entity) -> bool,
) -> HashSet<CellLevel> {
    let mut visible = HashSet::default();
    for fov in observers {
        // Conscious-observer gate (clause "only conscious player-faction observers
        // contribute"): a Downed / Dead observer reveals nothing. The caller restricts
        // `observers` to the PLAYER faction; this restricts to the ALIVE ones.
        if !fov.life.is_active() {
            continue;
        }
        let observer = Observer {
            position: fov.position,
            stance:   fov.stance,
            facing:   fov.facing,
        };
        // Scan ONLY the grid's authored/occupied cells (sparse content), filtered to
        // this observer's Chebyshev disc — never the disc's air (clause 5 / 6).
        for candidate in occupancy.authored_or_occupied_cells() {
            if chebyshev(fov.position, &candidate) > *tuning.view_range {
                continue;
            }
            // Resolve the candidate occupant's band the SAME way the shot pipeline does
            // (cover entry else published occupant band), and build the Target at that
            // band so `can_see` aims at the shot-pipeline anchor (clause 5; the
            // banded-occupant AC). The inert Standing stance is the documented no-band
            // fallback, mirroring `TargetGeometry::compose`.
            let position = Position::new(candidate);
            let stance = Stance::new(StanceKind::Standing);
            let target = Target {
                position: &position,
                stance:   &stance,
            };
            if *can_see(
                &observer,
                &target,
                fov.life,
                tuning.view_range,
                occupancy,
                surface,
                cover,
                tuning,
                &is_dead,
            ) {
                visible.insert(candidate);
            }
        }
    }
    visible
}

/// The 2D Chebyshev distance between an observer position and a candidate cell —
/// `max(|dx|, |dy|)` over the `(x, y)` ground plane only (the level/z axis is the LOS
/// probe's, never this term — the [`can_see`] disc convention).
///
/// A `u16` to match [`ViewRange`](crate::tuning::ViewRange)'s inner; the cell deltas on
/// the 60×60 grid fit it comfortably, saturating rather than wrapping on a pathological
/// out-of-grid delta.
fn chebyshev(observer: &Position, candidate: &CellLevel) -> u16 {
    // `Position` derefs `CellLevel` → the inner `IVec3`; `candidate` is a `CellLevel`.
    let dx = (observer.x - candidate.x).unsigned_abs();
    let dy = (observer.y - candidate.y).unsigned_abs();
    u16::try_from(dx.max(dy)).unwrap_or(u16::MAX)
}

/// Fold a freshly-computed VISIBLE set into a new squad fog — VISIBLE **replaces**, but
/// EXPLORED **only grows** (GTW-340 clause 4).
///
/// `explored_next = previous.explored ∪ visible_next` (monotone — EXPLORED accrues and
/// never removes, so a cell that leaves VISIBLE stays EXPLORED) and the VISIBLE set is
/// replaced wholesale by `visible_next`. A pure value transform — no I/O, no `Commands`,
/// no `&mut World` — the one GTW-341's recompute system calls to produce the next
/// [`SquadVisibility`] from the current one and the recomputed union.
#[must_use]
pub fn accrue(previous: &SquadVisibility, visible_next: HashSet<CellLevel>) -> SquadVisibility {
    // EXPLORED accrues monotonically: start from the previous mission memory and add
    // every newly-visible cell. The `visible` cells are by construction a subset of
    // `explored`, so the resulting pair holds the `visible ⊆ explored` invariant.
    let mut explored: HashSet<CellLevel> = previous.explored_cells().copied().collect();
    explored.extend(visible_next.iter().copied());
    SquadVisibility::new(visible_next, explored)
}
