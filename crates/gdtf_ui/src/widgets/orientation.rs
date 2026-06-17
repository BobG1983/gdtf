//! The shared [`Orientation`] axis enum for the layout-directional widgets.

use bevy::ui::FlexDirection;

/// The layout axis of a directional widget — a [`Switch`](super::Switch) knob track
/// or a [`SegmentedControl`](super::SegmentedControl) segment row.
///
/// A named, closed vocabulary rather than a bare boolean: a widget lays out along one
/// of two axes, and an enum says which at the call site (`Orientation::Vertical`)
/// instead of an opaque `true`/`false`. UI-level plumbing, not a game-domain value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
    /// Lay out along the X axis (left-to-right): a horizontal switch, a segment
    /// `Row`.
    #[default]
    Horizontal,
    /// Lay out along the Y axis (top-to-bottom): a vertical switch, a segment
    /// `Column` (the Stand/Kneel/Prone stance stack in the mockup).
    Vertical,
}

impl Orientation {
    /// The [`FlexDirection`](bevy::ui::FlexDirection) a flex container uses to lay its
    /// children out along this axis.
    #[must_use]
    pub const fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }
}
