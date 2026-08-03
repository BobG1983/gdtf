use bevy::prelude::*;

crate::support_item! {
                                                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContextualPanelRoot;
}

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_Z: i32 = 20;

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_RIGHT_VW: f32 = 1.5;

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_WIDTH_VW: f32 = 22.0;

pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_ROW_GAP_VH: f32 = 1.0;
