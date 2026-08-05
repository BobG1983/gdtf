//! Composite the offscreen capture image back onto the window.

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};

use super::target::QaCaptureTarget;
use crate::capture::aims_at;

/// Render layer nothing but the present pass draws to.
pub const PRESENT_LAYER: usize = 2;

/// Camera order that composites the capture above every other camera.
pub const PRESENT_ORDER: isize = 100;

/// Camera that composites the capture image onto the window.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentCamera;

/// Sprite showing the capture image on the present layer.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentSprite;

pub(super) fn spawn_present_pass(
    target: Option<Res<QaCaptureTarget>>,
    existing: Query<(), With<PresentCamera>>,
    cameras: Query<&RenderTarget, With<Camera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    // Compositing an untouched image over a live window would blank it.
    if !cameras.iter().any(|current| aims_at(current, &target)) {
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
        PresentCamera,
    ));
    commands.spawn((
        Sprite {
            image: target.handle.clone(),
            custom_size: Some(window.size()),
            ..default()
        },
        layers,
        PresentSprite,
    ));
}
