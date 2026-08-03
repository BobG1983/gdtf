use bevy::ui::FlexDirection;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
            #[default]
    Horizontal,
            Vertical,
}

impl Orientation {
            #[must_use]
    pub const fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }
}
