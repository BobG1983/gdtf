use bevy::{color::Alpha, prelude::*, ui::BackgroundColor};

use crate::states::running::game::battlescape::bottom_bar::components::{
    BottomBarRoot, bottom_bar_padding,
};

pub(in crate::states::running::game::battlescape) fn opacify_bottom_bar(
    mut bars: Query<&mut BackgroundColor, With<BottomBarRoot>>,
) {
    for mut fill in &mut bars {
        if fill.0.alpha() < 1.0 {
            fill.0.set_alpha(1.0);
        }
    }
}

pub(in crate::states::running::game::battlescape) fn repad_bottom_bar(
    mut bars: Query<&mut Node, With<BottomBarRoot>>,
) {
    let want = bottom_bar_padding();
    for mut node in &mut bars {
        if node.padding != want {
            node.padding = want;
        }
    }
}
