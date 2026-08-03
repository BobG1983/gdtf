use bevy::prelude::*;

crate::support_item! {
                                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct BottomBarRoot;
}

pub(in crate::states::running::game::battlescape) const BOTTOM_BAR_H_VH: f32 = 30.0;

pub(in crate::states::running::game::battlescape) const BOTTOM_BAR_PAD_Y_VH: f32 = 1.5;

pub(in crate::states::running::game::battlescape) const BOTTOM_BAR_PAD_X_VW: f32 = 0.8;

pub(in crate::states::running::game::battlescape) const fn bottom_bar_padding() -> UiRect {
    UiRect {
        left:   Val::Vw(BOTTOM_BAR_PAD_X_VW),
        right:  Val::Vw(BOTTOM_BAR_PAD_X_VW),
        top:    Val::Vh(BOTTOM_BAR_PAD_Y_VH),
        bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
    }
}
