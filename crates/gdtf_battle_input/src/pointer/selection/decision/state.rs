//! Selection state a click reads and then updates.

use bevy::prelude::*;

use crate::{
    InspectTarget,
    selection::{path_preview::PathPreviewTarget, resources::SelectedShooter},
};

/// Selected shooter, inspect pin, and pinned move target.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PointerSelection<'w> {
    /// Currently selected shooter.
    pub(in crate::pointer) selected:    ResMut<'w, SelectedShooter>,
    /// Hovered and pinned inspect target.
    pub(in crate::pointer) inspect:     ResMut<'w, InspectTarget>,
    /// Cell the move path preview is pinned to.
    pub(in crate::pointer) move_target: ResMut<'w, PathPreviewTarget>,
}

impl PointerSelection<'_> {
    /// The hovered and pinned inspect target a click decision reads.
    #[must_use]
    pub fn inspect(&self) -> &InspectTarget {
        &self.inspect
    }

    /// The inspect target, for a caller that sets the hover before deciding.
    pub fn inspect_mut(&mut self) -> &mut InspectTarget {
        &mut self.inspect
    }
}
