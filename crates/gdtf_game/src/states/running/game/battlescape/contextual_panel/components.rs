use bevy::prelude::*;

crate::support_item! {
    /// Root node of the contextual act panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContextualPanelRoot;
}

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_WIDTH_PCT: f32 = 22.4;

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_ROW_GAP_VH: f32 = 1.0;
