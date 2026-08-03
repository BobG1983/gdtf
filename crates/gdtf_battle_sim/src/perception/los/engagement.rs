//! Engagement visibility: alive, in range, and clear LOS.

use bevy::prelude::{Deref, Entity};

use super::probe::{Observer, Sighted, Target, has_los};
use crate::{
    cover::CoverLedger,
    ganger::{LifeState, Position},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::{CombatTuning, ViewRange},
};

/// Whether the observer can currently see the target for engagement.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanSee(bool);

impl CanSee {
    /// Wrap a can-see flag.
    #[must_use]
    pub const fn new(can_see: bool) -> Self {
        Self(can_see)
    }
}

/// True when the observer is alive, the target is within view range, and LOS is clear.
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
    if !*observer_life.is_active() {
        return CanSee::new(false);
    }

    if *chebyshev(observer.position, target.position) > *view_range {
        return CanSee::new(false);
    }

    let sighted: Sighted = has_los(observer, target, occupancy, surface, cover, tuning, is_dead);
    CanSee::new(*sighted)
}

fn chebyshev(a: &Position, b: &Position) -> TargetRange {
    let da = (a.x - b.x).unsigned_abs();
    let db = (a.y - b.y).unsigned_abs();
    let max = da.max(db);
    TargetRange::new(u16::try_from(max).unwrap_or(u16::MAX))
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TargetRange(u16);

impl TargetRange {
    const fn new(range: u16) -> Self {
        Self(range)
    }
}
