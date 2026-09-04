use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::WorldCamera;

use crate::states::running::game::battlescape::BottomBarRoot;

/// non-negative integer well within `u32`; the localized `#[expect]` (the framing.rs
const fn physical_px(value: f32) -> u32 {
    value.round() as u32
}

pub(in crate::states::running::game::battlescape) fn set_world_viewport(
    mut cameras: Query<&mut Camera, With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    bottom_bars: Query<&ComputedNode, With<BottomBarRoot>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let win = window.physical_size();

    let bottom = bottom_bars
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().y));

    let physical_position = UVec2::ZERO;
    let physical_size = UVec2::new(win.x, win.y.saturating_sub(bottom));

    for mut camera in &mut cameras {
        let unchanged = camera.viewport.as_ref().is_some_and(|current| {
            current.physical_position == physical_position && current.physical_size == physical_size
        });
        if unchanged {
            continue;
        }
        camera.viewport = Some(Viewport {
            physical_position,
            physical_size,
            depth: 0.0..1.0,
        });
    }
}
