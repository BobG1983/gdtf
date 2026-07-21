//! Retarget the game's world + UI cameras to the offscreen capture image (GTW-764).
//!
//! A `bevy_ui` node tree renders to exactly ONE camera, so the HUD only appears in the
//! capture if the UI camera itself renders into the offscreen image — hence BOTH the world
//! camera and the UI camera are retargeted, and a separate present pass ([`super::blit`])
//! shows the image on the window.

use bevy::{camera::RenderTarget, prelude::*, ui::IsDefaultUiCamera};
use gdtf_battle_presenter::WorldCamera;

use super::target::QaCaptureTarget;
use crate::states::running::UiCamera;

/// The [`Query`] filter selecting the game's world + UI cameras. A `type` alias so the system
/// signature stays under clippy's `type_complexity` ceiling.
type GameCameras = Or<(With<WorldCamera>, With<UiCamera>)>;

/// `Update`: point every world or UI camera's `RenderTarget` at the offscreen
/// [`QaCaptureTarget`] image (GTW-764).
///
/// A `Camera` `#[require]`s a `RenderTarget` (defaulting to `RenderTarget::Window`), so a
/// freshly-spawned camera already carries one aimed at the window; this OVERWRITES it with
/// `RenderTarget::Image`. IDEMPOTENT and re-applied to newly-spawned cameras: a camera already
/// aimed at the offscreen image is skipped, and a camera spawned later — the world camera on
/// `OnEnter(GameState::BattleScape)`, the UI camera on `OnEnter(AppState::Running)` — is picked
/// up the next frame (its target is still the window). Only the render target changes; each
/// camera's order / clear-config / render layers / [`Camera::viewport`] are untouched, so
/// `set_world_viewport` keeps working (the image size equals the window size, so the
/// physical-pixel viewport rect stays correct).
///
/// No-ops until the [`QaCaptureTarget`] exists (it is created a frame or two after startup).
/// Param-only (`bevy-traps.md` #7): a filtered `Query` + `Commands`, no `&mut World`.
pub(in crate::dev::net_qa) fn retarget_cameras_to_offscreen(
    target: Option<Res<QaCaptureTarget>>,
    cameras: Query<(Entity, &RenderTarget), GameCameras>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    let handle = &**target;
    for (entity, current) in &cameras {
        // Already aimed at the offscreen image — nothing to do (keeps the write idempotent so
        // it does not re-dirty the camera every frame).
        if current.as_image() == Some(handle) {
            continue;
        }
        commands
            .entity(entity)
            .insert(RenderTarget::Image(handle.clone().into()));
    }
}

/// `Update`: mark the [`UiCamera`] as the default UI camera so the `bevy_ui` HUD renders into
/// the offscreen capture image, not the present camera (GTW-764).
///
/// Absent an explicit `IsDefaultUiCamera` / `UiTargetCamera`, `bevy_ui` binds every UI root to
/// the HIGHEST-ORDER camera targeting the window. GTW-764 adds an order-`100` present camera
/// (which renders only its own [`RenderLayers`], not the layer-0 UI), so without this the HUD
/// would bind to the present camera and be culled — rendering NOWHERE. Marking the [`UiCamera`]
/// (which now renders into the offscreen image) makes the HUD composite into the capture.
///
/// Only the [`UiCamera`] is ever marked — never the world or present camera, so `bevy_ui` never
/// sees two `IsDefaultUiCamera` cameras (which it warns on). IDEMPOTENT via the
/// `Without<IsDefaultUiCamera>` filter (the marker is not a required component, so this
/// correctly detects absence); re-applied to a UI camera spawned later. This is `net_qa`-gated
/// (the plugin only adds it on the env-active path), so normal play still has just the world +
/// UI cameras and resolves the HUD to the order-`0` UI camera exactly as before.
///
/// Param-only (`bevy-traps.md` #7): a filtered `Query` + `Commands`, no `&mut World`.
pub(in crate::dev::net_qa) fn mark_ui_default_camera(
    ui_cameras: Query<Entity, (With<UiCamera>, Without<IsDefaultUiCamera>)>,
    mut commands: Commands,
) {
    for entity in &ui_cameras {
        commands.entity(entity).insert(IsDefaultUiCamera);
    }
}
