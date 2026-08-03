//! Horizontal vs vertical layout for widgets.

use bevy::ui::FlexDirection;

/// Widget axis.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
    /// Left to right.
    #[default]
    Horizontal,
    /// Top to bottom.
    Vertical,
}

impl Orientation {
    /// Matching Bevy flex direction.
    #[must_use]
    pub const fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }
}
