//! The single-observer **engagement gate** — [`can_see`], the composition that decides
//! whether one ganger can engage another (GTW-339, leaf 3 of the GTW-13 FOV epic).
//!
//! `can_see` is the one place the FOV epic composes its three independent checks:
//! the **conscious-observer** gate ([`LifeState::is_active`](crate::ganger::LifeState::is_active)),
//! the **range disc** (a 2D Chebyshev bound on the observer/target cells), and the
//! level-aware **line-of-sight probe** ([`has_los`]). It is pure — render-free, RNG-free,
//! deterministic, no `&mut World` / `Commands` — and reuses [`has_los`] verbatim, so the
//! shot-pipeline band + facing-neutral eye resolved in GTW-337 hold here too.

use bevy::prelude::{Deref, Entity};

use super::probe::{Observer, Sighted, Target, has_los};
use crate::{
    cover::CoverLedger,
    ganger::{LifeState, Position},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::{CombatTuning, ViewRange},
};

/// Whether one ganger can **engage** another this instant — the verdict
/// [`can_see`] returns (`true` = in range, conscious, with clear LOS; `false`
/// otherwise).
///
/// A named newtype over `bool` (no-bare-types: an engagement verdict is a domain
/// value, not a bare boolean), mirroring [`Sighted`]. Private inner + derived
/// [`Deref`] (house style): read the verdict through the `*` deref, construct it
/// only through [`CanSee::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanSee(bool);

impl CanSee {
    /// Build an engagement verdict — `true` when the observer is a conscious watcher,
    /// the target is inside the Chebyshev range disc, AND line of sight is clear;
    /// `false` otherwise.
    #[must_use]
    pub const fn new(can_see: bool) -> Self {
        Self(can_see)
    }
}

/// The single-observer **engagement gate**: can this `observer` SEE-AND-ENGAGE this
/// `target` right now? (GTW-339, leaf 3 of the GTW-13 FOV epic.)
///
/// Composes the FOV epic's three checks, short-circuiting on the cheapest first:
///
/// 1. **Conscious-observer gate.** Returns `false` immediately unless the observer's
///    [`LifeState::is_active`](crate::ganger::LifeState::is_active) — only an
///    [`Alive`](crate::ganger::LifeState::Alive) ganger watches; a `Downed` or `Dead`
///    observer SEES NOTHING regardless of range or LOS.
/// 2. **Range disc.** The 2D Chebyshev distance — `max(|dx|, |dy|)` over the
///    observer/target **cell** `(x, y)` — must be `<= *view_range`. This is the disc
///    that bounds range; the level/z axis is owned by the LOS probe below, NOT this
///    term (so z never enters the Chebyshev max). `view_range` is the named
///    [`ViewRange`] tunable the caller reads off [`CombatTuning`] — never hardcoded.
/// 3. **Line of sight.** [`has_los`] must report a clear line — called **exactly
///    once** (no second probe), so the shot-pipeline aim band and the facing-neutral
///    eye resolved in GTW-337 hold here verbatim, along with the corpse `is_dead`
///    pass-through.
///
/// `can_see` is **faction-agnostic at the function level**: the same composition is
/// consulted per `(observer, target)` pair by the AI engagement gate (GTW-70) and by
/// reaction fire (GTW-38) — it is NOT player-only. The player-faction restriction (the
/// squad-union "what the *player* squad collectively sees") lives in the squad-union
/// writer (GTW-340), NOT here.
///
/// Pure: render-free, RNG-free, deterministic, no `&mut World` / `Commands`. The
/// `is_dead` predicate is the same read-only corpse predicate [`has_los`] takes (a
/// `Dead` occupant does not block the sight ray).
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the engagement gate composes every input its three checks need — the \
              observer/target borrow-views, the observer's life state, the view-range \
              tunable, the three grids + tuning has_los marches, and the corpse \
              predicate; bundling them into a struct would only hide the same arity"
)]
pub fn can_see(
    observer: &Observer,
    target: &Target,
    observer_life: LifeState,
    view_range: ViewRange,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: impl Fn(Entity) -> bool,
) -> CanSee {
    // 1. Conscious-observer gate (clause 2): a Downed / Dead watcher sees nothing.
    if !*observer_life.is_active() {
        return CanSee::new(false);
    }

    // 2. Range disc (clause 2/3): the 2D Chebyshev bound, x/y only — the level axis is
    //    the LOS probe's, never this term. `<=` so the edge cell is inclusive.
    if *chebyshev(observer.position, target.position) > *view_range {
        return CanSee::new(false);
    }

    // 3. Line of sight (clause 4): the ONE probe — reused verbatim, so the GTW-337
    //    band + facing-neutral eye hold here too.
    let sighted: Sighted = has_los(observer, target, occupancy, surface, cover, tuning, is_dead);
    CanSee::new(*sighted)
}

/// The 2D Chebyshev distance between two grid positions — `max(|dx|, |dy|)` over the
/// **cell** `(x, y)` only (GTW-339 clause 3).
///
/// This is the disc that bounds engagement *range*; the level/z axis is deliberately
/// excluded — the LOS probe owns the height axis, so a one-storey climb never inflates
/// the range term. A `u16` to match [`ViewRange`]'s inner: the `|dx|`/`|dy|` magnitudes
/// fit the 60×60 grid comfortably.
fn chebyshev(a: &Position, b: &Position) -> TargetRange {
    // `Position` derefs to `CellLevel`, which derefs to the inner `IVec3` (x, y, z).
    let da = (a.x - b.x).unsigned_abs();
    let db = (a.y - b.y).unsigned_abs();
    let max = da.max(db);
    // Cell deltas on the 60×60 grid fit a u16 with room to spare; saturate rather than
    // wrap on the pathological out-of-grid input (defined, never a panic).
    TargetRange::new(u16::try_from(max).unwrap_or(u16::MAX))
}

/// The **2D range to a target** — the Chebyshev cell-distance `max(|dx|, |dy|)`
/// between two combatants' `(x, y)` cells, the value the engagement range disc bounds
/// against [`ViewRange`] (GTW-339 clause 3).
///
/// A named domain value (no bare `u16`): it is how far apart two gangers stand on the
/// grid, distinct from the [`ViewRange`] *config* it is compared to (a distance, not a
/// sight limit). Private inner + derived [`Deref`]; build one via [`TargetRange::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TargetRange(u16);

impl TargetRange {
    /// Build a target range from its Chebyshev cell-distance magnitude.
    const fn new(range: u16) -> Self {
        Self(range)
    }
}
