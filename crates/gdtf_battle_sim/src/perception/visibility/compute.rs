//! The pure value-transform helpers behind the squad fog — [`union_fov`] (the
//! squad VISIBLE union over a bounded candidate set) and [`accrue`] (the monotone
//! VISIBLE-replaces / EXPLORED-grows fold). NO I/O, no `Commands`, no `&mut World`:
//! these are the value transforms GTW-341's recompute system calls (GTW-340, leaf 4).

use bevy::{platform::collections::HashSet, prelude::Deref};

use crate::{
    cover::CoverLedger,
    ganger::{Facing, LifeState, Position, Stance, StanceKind},
    los::{Observer, PeekOffset, Target, can_see},
    metric::{Cell, CellLevel, CellUnit, Level},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, StairEyeOffset},
    surface::SurfaceGrid,
    tuning::{CombatTuning, ViewRange},
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
/// per candidate, plus the authored stair-tile eye-lift (GTW-390) for the observer's
/// cell.
#[derive(Debug, Clone, Copy)]
pub struct FovObserver<'a> {
    /// The observer's `(cell, level)` grid position — the disc center + eye datum.
    pub position:         &'a Position,
    /// The observer's stance — selects the per-stance muzzle level-fraction the eye
    /// sits at (the same anchor [`can_see`] / [`has_los`](crate::los::has_los) use).
    pub stance:           &'a Stance,
    /// The observer's facing — carried to build the [`Observer`] view; the
    /// facing-neutral eye does not read it (FOV is omni-directional).
    pub facing:           &'a Facing,
    /// The observer's life state — only [`LifeState::is_active`] (Alive) contributes;
    /// an inactive observer is skipped.
    pub life:             LifeState,
    /// The authored stair-tile eye-lift for the observer's cell (GTW-390) — looked
    /// up from [`OccupancyGrid::stair_eye_offset_at`] by the caller
    /// ([`recompute_visibility`](crate::visibility::recompute_visibility)) and
    /// threaded in here so the pure probe stays resource-free.
    pub stair_eye_offset: StairEyeOffset,
}

/// The squad **VISIBLE** union over a **disc-bounded DENSE scan** — every `(cell, level)`
/// some conscious player-faction `observers` member currently sees (GTW-340 clause 5,
/// GTW-347 regression fix).
///
/// Candidate set (concrete, bounded — clause 1 / 3): for each conscious observer, scan
/// its Chebyshev disc **densely** — every `(x, y)` with `max(|dx|, |dy|) <= *view_range`,
/// clamped to the grid extent (`0..GRID_WIDTH` × `0..GRID_HEIGHT`) — across the map's
/// **authored level range** ([`OccupancyGrid::authored_level_range`]) UNIONED with the
/// observer's own storey. This is a dense disc scan, NOT the sparse authored/occupied
/// subset: the presenter floors EVERY in-range open cell (`role_at` maps
/// [`TerrainKind::Open`](crate::occupancy::TerrainKind) → floor), so the rendered terrain
/// layer is the WHOLE active-level grid — the fog mask must reveal the same dense floor
/// the view draws (GTW-347; the pre-fix sparse scan revealed only authored/occupied cells,
/// so open floor was never revealed and `present_fog` hid the whole map).
///
/// The level axis is the authored range (so a single-level map collapses to that one
/// storey, never a blind `0..`[`MAX_LEVELS`](crate::metric::MAX_LEVELS)) unioned with the
/// observer's storey — an observer on a flat all-Open floor with NO authored content still
/// reveals the cells on its own level. The `x`/`y` bound mirrors the presenter's
/// `draw_static_battlefield` per-level `0..GRID_WIDTH` × `0..GRID_HEIGHT` scan, intersected
/// with the disc, so the fog and the rendered layer cover the same cells.
///
/// For each candidate, the occupant's band is resolved the EXACT way the shot pipeline
/// resolves it (inside [`can_see`] → [`has_los`](crate::los::has_los):
/// `cover.peek(at).map(height_band).or_else(|| occupancy.occupant_band(at))`), and the
/// [`Target`] view is built with an inert [`StanceKind::Standing`] fallback (mirroring the
/// shot pipeline) before [`can_see`] runs. [`can_see`] applies the SAME range disc + LOS
/// probe — so an out-of-sight or LOS-blocked cell is correctly excluded even though the
/// disc scan offered it. The [`can_see`]-true candidates fold into one VISIBLE set.
///
/// Read-only over the grids; render-free, RNG-free, deterministic, no `&mut World` /
/// `Commands`. The `is_dead` corpse predicate is the same read-only pass-through
/// [`can_see`] / [`has_los`](crate::los::has_los) take.
///
/// PERF (clause 3): the per-step recompute cost is **the disc** — `~(2·view_range + 1)²`
/// `can_see` probes per observer per authored level, never the full `60 × 60 × 8` grid
/// (the disc is clamped to the grid extent and bounded to the authored level range). The
/// recompute (GTW-341) rides EVERY accepted `Changed<Position>` move step, so this scan
/// MUST stay disc-bounded. NOTE: a shadowcasting FOV (an `O(perimeter)` sweep instead of
/// the `O(disc area)` per-cell probe) is a DEFERRED future optimization — it is NOT built
/// here; the disc-bounded per-cell probe is the shipped GTW-347 behaviour.
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
    // The map's authored storey range (None on an entirely empty grid) — the level span
    // the dense disc scans, unioned per observer with that observer's own storey.
    let authored = occupancy.authored_level_range();
    for fov in observers {
        // Conscious-observer gate (clause "only conscious player-faction observers
        // contribute"): a Downed / Dead observer reveals nothing. The caller restricts
        // `observers` to the PLAYER faction; this restricts to the ALIVE ones.
        if !*fov.life.is_active() {
            continue;
        }
        let observer = Observer {
            position:         fov.position,
            stance:           fov.stance,
            facing:           fov.facing,
            stair_eye_offset: fov.stair_eye_offset,
            // C4 union-peek-free invariant (GTW-393): the squad fog union always
            // uses the centred eye — peek only enters a targeted per-shot query.
            // This literal MUST NOT gain a `peek_offset: fov.peek_offset` binding.
            peek_offset:      PeekOffset::default(),
        };
        // Scan the observer's Chebyshev disc DENSELY over the authored level range (∪ the
        // observer's own storey), clamped to the grid extent — the dense floor the
        // presenter draws (clause 1 / 3). `can_see` re-applies the range disc + LOS, so
        // an out-of-sight cell offered by the (square) disc bound is excluded.
        for (level, cell) in disc_cells(fov.position, tuning.view_range, authored) {
            let candidate = CellLevel::new(cell, level);
            // Build the Target at the candidate cell. The occupant's band is resolved
            // inside `can_see` / `has_los` the SAME way the shot pipeline does (cover
            // entry else published occupant band); the inert Standing stance is the
            // documented no-band fallback (the banded-occupant AC).
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

/// Enumerate the `(level, cell)` candidates of one observer's dense Chebyshev disc — every
/// `(x, y)` with `max(|dx|, |dy|) <= view_range`, clamped to the grid extent
/// (`0..GRID_WIDTH` × `0..GRID_HEIGHT`), across the `authored` level range unioned with the
/// observer's own storey (GTW-347 clause 1 / 3).
///
/// The `x`/`y` window is the observer's cell ± `view_range`, clamped to the grid so it
/// never offers an off-grid cell (mirroring the presenter's per-level
/// `draw_static_battlefield` scan). The level span is `authored` (the map's authored storey
/// range) widened to include the observer's storey, so a single-level map scans one storey
/// and a flat all-Open floor (no authored content) still scans the observer's level. The
/// caller hands each `(level, cell)` to [`can_see`], which re-applies the exact disc + LOS,
/// so the corner cells of the square `x`/`y` window outside the Chebyshev radius and any
/// blocked cell are excluded there.
fn disc_cells(
    observer: &Position,
    view_range: ViewRange,
    authored: Option<(Level, Level)>,
) -> impl Iterator<Item = (Level, Cell)> {
    let radius = i32::from(*view_range);
    // The observer's own storey is always in the scan (a flat all-Open floor has no
    // authored content, yet the squad still sees its own level — clause 1 / 4). The
    // canonical CellLevel::level accessor through Position's deref (GTW-565).
    let observer_level = observer.level();
    let (lo, hi) = match authored {
        None => (observer_level, observer_level),
        Some((lo, hi)) => (lo.min(observer_level), hi.max(observer_level)),
    };
    // The x/y window: the observer's cell ± view_range, clamped to the grid extent so no
    // off-grid (x, y) is ever offered (the presenter never draws one either).
    let x_min = (observer.x - radius).max(0);
    let x_max = (observer.x + radius).min(*i32_extent(GridExtent::new(GRID_WIDTH)) - 1);
    let y_min = (observer.y - radius).max(0);
    let y_max = (observer.y + radius).min(*i32_extent(GridExtent::new(GRID_HEIGHT)) - 1);
    (*lo..=*hi).flat_map(move |level_index| {
        (y_min..=y_max).flat_map(move |y| {
            (x_min..=x_max).map(move |x| (Level::new(level_index), Cell::new(x, y)))
        })
    })
}

/// A **grid extent** — the width or height of the coarse cell grid, in whole cells
/// (`GRID_WIDTH` / `GRID_HEIGHT`, both `60`).
///
/// Names the slot-buffer dimension so [`i32_extent`] does not take a bare `usize`
/// (no-bare-types). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct GridExtent(usize);

impl GridExtent {
    /// Build a grid extent from its whole-cell dimension.
    #[must_use]
    const fn new(extent: usize) -> Self {
        Self(extent)
    }
}

/// The grid [`GridExtent`] as a [`CellUnit`] coordinate bound — the inclusive upper
/// edge of the `x`/`y` disc clamp.
///
/// `GRID_WIDTH` / `GRID_HEIGHT` are `60`, so the conversion is always in range; a
/// pathological oversize saturates at [`i32::MAX`] rather than wrapping (defined, never a
/// panic).
fn i32_extent(dimension: GridExtent) -> CellUnit {
    CellUnit::new(i32::try_from(*dimension).unwrap_or(i32::MAX))
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
