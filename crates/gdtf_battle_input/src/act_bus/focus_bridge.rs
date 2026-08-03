//! Panel navigation order and focused panel button lookup.

use bevy::{input_focus::InputFocus, prelude::*};

/// Sort key for panel button focus navigation.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PanelNavOrder(u16);

impl PanelNavOrder {
    /// Build from a navigation order index.
    #[must_use]
    pub const fn new(order: u16) -> Self {
        Self(order)
    }
}

/// Return the focused entity if it carries [`PanelNavOrder`].
#[must_use]
pub fn focused_panel_button(
    focus: Option<&InputFocus>,
    panels: &Query<(), With<PanelNavOrder>>,
) -> Option<Entity> {
    let focused = focus?.get()?;
    panels.contains(focused).then_some(focused)
}
