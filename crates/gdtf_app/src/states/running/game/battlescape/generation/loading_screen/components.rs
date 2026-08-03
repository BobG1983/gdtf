use bevy::prelude::*;

crate::support_item! {
                                                                #[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct LoadingScreenRoot;
}

pub(in crate::states::running::game::battlescape::generation) const LOADING_SCREEN_Z: i32 = 1000;

pub(in crate::states::running::game::battlescape::generation) const LOADING_BOX_W_VW: f32 = 40.0;

pub(in crate::states::running::game::battlescape::generation) const LOADING_BOX_MIN_H_VH: f32 =
    12.0;
