use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};

use super::target::QaCaptureTarget;

const PRESENT_LAYER: usize = 2;

const PRESENT_ORDER: isize = 100;

#[derive(Component, Debug, Clone, Copy)]
pub(in crate::dev::net_qa) struct QaPresentCamera;

#[derive(Component, Debug, Clone, Copy)]
struct QaPresentSprite;

pub(in crate::dev::net_qa) fn spawn_present_pass(
    target: Option<Res<QaCaptureTarget>>,
    existing: Query<(), With<QaPresentCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    if !existing.is_empty() {
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
        QaPresentCamera,
    ));
    commands.spawn((
        Sprite {
            image: (**target).clone(),
            custom_size: Some(window.size()),
            ..default()
        },
        layers,
        QaPresentSprite,
    ));
}
