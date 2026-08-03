use bevy::{input_focus::InputFocus, prelude::*};

#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PanelNavOrder(u16);

impl PanelNavOrder {
        #[must_use]
    pub const fn new(order: u16) -> Self {
        Self(order)
    }
}

#[must_use]
pub fn focused_panel_button(
    focus: Option<&InputFocus>,
    panels: &Query<(), With<PanelNavOrder>>,
) -> Option<Entity> {
    let focused = focus?.get()?;
    panels.contains(focused).then_some(focused)
}
