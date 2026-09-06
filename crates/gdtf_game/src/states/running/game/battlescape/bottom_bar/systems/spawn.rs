use bevy::{
    color::Alpha,
    prelude::*,
    ui::{BackgroundColor, GlobalZIndex, Node, PositionType, UiRect, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::bottom_bar::components::{
    BOTTOM_BAR_GAP_X_VW, BOTTOM_BAR_H_VH, BottomBarRoot, bottom_bar_padding,
};

const BOTTOM_BAR_Z: i32 = 10;

pub(in crate::states::running::game::battlescape) fn spawn_bottom_bar(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let bar = spawn_panel(&mut commands, &theme);
    let mut opaque_fill = *theme.panel.color;
    opaque_fill.set_alpha(1.0);
    commands.entity(bar).insert((
        BottomBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::ZERO,
            left: Val::ZERO,
            width: Val::Vw(100.0),
            height: Val::Vh(BOTTOM_BAR_H_VH),
            border: UiRect::all(Val::Vw(*theme.panel.border_width)),
            padding: bottom_bar_padding(),
            column_gap: Val::Vw(BOTTOM_BAR_GAP_X_VW),
            ..default()
        },
        BackgroundColor(opaque_fill),
        GlobalZIndex(BOTTOM_BAR_Z),
    ));
}

pub(in crate::states::running::game::battlescape) fn despawn_bottom_bar(
    mut commands: Commands,
    bars: Query<Entity, With<BottomBarRoot>>,
) {
    for bar in &bars {
        commands.entity(bar).despawn();
    }
}
