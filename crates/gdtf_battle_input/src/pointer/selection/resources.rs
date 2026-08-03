use bevy::prelude::*;

#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedShooter(Option<Entity>);

impl SelectedShooter {
        #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

                #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

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
