//! Selected shooter resource and selection highlight marker.

use bevy::prelude::*;

/// Currently selected player ganger, if any.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedShooter(Option<Entity>);

impl SelectedShooter {
    /// Select this entity.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// No selection.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

/// Marker on the selection highlight overlay entity.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SelectionHighlight;

pub(super) const SELECTION_TINT: Color = Color::srgba(0.4, 0.85, 1.0, 0.5);

pub(super) fn set_selection(selected: &mut ResMut<SelectedShooter>, next: SelectedShooter) {
    if **selected != next {
        **selected = next;
    }
}

pub(super) fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
