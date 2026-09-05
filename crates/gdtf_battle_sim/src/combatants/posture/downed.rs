//! Lay a ganger prone the moment it can no longer act.

use bevy::prelude::{Changed, Query};

use crate::ganger::{LifeState, Stance, StanceKind};

type NewlyInactive<'world, 'state> =
    Query<'world, 'state, (&'static LifeState, &'static mut Stance), Changed<LifeState>>;

/// Write a prone stance on every ganger whose life state changed to downed or dead.
pub fn lay_inactive_prone(mut newly_inactive: NewlyInactive) {
    for (life, mut stance) in &mut newly_inactive {
        if !*life.is_active() {
            *stance = Stance::new(StanceKind::Prone);
        }
    }
}
