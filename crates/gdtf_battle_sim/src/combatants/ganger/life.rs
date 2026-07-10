//! The terminal life-state machine — the [`LifeState`] two-pool outcome. The §9
//! bleed-out state is the removable [`BleedingOut`](crate::effects::bleed::BleedingOut)
//! condition (GTW-695), owned by the bleed module — not a per-ganger flag here.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// Whether a ganger is a **conscious observer / actor** — `true` only while
/// [`Alive`](LifeState::Alive) ([`LifeState::is_active`]).
///
/// The squad-FOV `is_active` gate (`docs/combat/visibility.md`): an Alive ganger
/// contributes sight and may act; a Downed or Dead one does neither. A distinct
/// agency predicate, not a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active(bool);

impl Active {
    /// Build the agency verdict from the life-state test.
    #[must_use]
    pub const fn new(active: bool) -> Self {
        Self(active)
    }
}

/// A ganger's terminal life state — the two-pool outcome machine.
///
/// The combat/death system owns this (it is a state component, not a stat):
/// `HP ≤ 0` → [`Downed`](LifeState::Downed) (alive, incapacitated, dying),
/// `Wounds ≤ 0` → [`Dead`](LifeState::Dead) (Dead trumps Downed). It drives
/// occupancy updates in E1.7 (a corpse / downed body frees or holds its cell
/// differently): a [`Downed`](LifeState::Downed) ganger is a body still on the
/// field — it HOLDS its cell (it keeps its `Position`, blocks movement, and
/// occludes fire); only a [`Dead`](LifeState::Dead) ganger FREES its cell (the
/// corpse is cleared from the occupancy grid by
/// [`sync_dead_gangers`](crate::occupancy_sync::sync_dead_gangers), then despawned).
/// GTW-459. A standalone named enum component (wounds-and-roster.md state machine).
/// Defaults to [`Alive`](LifeState::Alive).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum LifeState {
    /// Up and fighting — full agency.
    #[default]
    Alive,
    /// HP gone, Wounds remaining — incapacitated but alive (bleeding out unless
    /// stabilized; can be executed or recovered).
    Downed,
    /// Wounds gone — dead in battle (from stacked injuries or one Fatal hit).
    Dead,
}

impl LifeState {
    /// Whether this ganger is a **conscious observer** — `true` ONLY for
    /// [`Alive`](LifeState::Alive).
    ///
    /// The squad-FOV epic's `is_active` gate (GTW-13 / `docs/combat/visibility.md`):
    /// a ganger contributes sight only while it is up and aware. A
    /// [`Downed`](LifeState::Downed) ganger is incapacitated (bleeding out, no agency)
    /// and a [`Dead`](LifeState::Dead) one is a corpse — neither watches, so both
    /// SEE NOTHING. [`can_see`](crate::los::can_see) consults this first: an inactive
    /// observer fails the gate regardless of range or line of sight.
    #[must_use]
    pub const fn is_active(self) -> Active {
        Active::new(matches!(self, Self::Alive))
    }
}
