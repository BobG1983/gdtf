use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};
use bevy_egui::PrimaryEguiContext;

use super::target::{EditorQaCaptureTarget, aims_at};

/// prefab preview camera's layer `1` (`crate::preview::target::PREVIEW_LAYER`), so the present
pub(in crate::net_qa) const PRESENT_LAYER: usize = 2;

pub(in crate::net_qa) const PRESENT_ORDER: isize = 100;

#[derive(Component, Debug, Clone, Copy)]
pub(in crate::net_qa) struct EditorQaPresentCamera;

#[derive(Component, Debug, Clone, Copy)]
pub(in crate::net_qa) struct EditorQaPresentSprite;

pub(in crate::net_qa) fn spawn_editor_present_pass(
    target: Option<Res<EditorQaCaptureTarget>>,
    existing: Query<(), With<EditorQaPresentCamera>>,
    egui_cameras: Query<&RenderTarget, With<PrimaryEguiContext>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    if !egui_cameras.iter().any(|current| aims_at(current, &target)) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let layers = RenderLayers::layer(PRESENT_LAYER);
    commands.spawn((
        Camera2d,
        Camera {
            order: PRESENT_ORDER,
            ..default()
        },
        RenderTarget::Window(WindowRef::Primary),
        layers.clone(),
        EditorQaPresentCamera,
    ));
    commands.spawn((
        Sprite {
            image: target.handle.clone(),
            custom_size: Some(window.size()),
            ..default()
        },
        layers,
        EditorQaPresentSprite,
    ));
}
