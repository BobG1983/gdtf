//! Plugin that captures the window's frame without retargeting any camera.

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::ExtractResourcePlugin,
        renderer::{RenderGraph, RenderGraphSystems},
        view::{prepare_view_attachments, prepare_view_targets},
    },
};

use super::{
    attachment::override_window_attachment,
    blit::blit_capture_to_window,
    png::write_capture_png,
    target::{CaptureImage, CaptureWindowTarget, sync_capture_target},
};
use crate::capture::CaptureSystems;

/// Captures the primary window's frame by overriding its output color attachment.
///
/// Every camera stays on the window, so a capture never disturbs UI interaction.
pub struct WindowCapturePlugin;

impl Plugin for WindowCapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptureImage>();
        app.add_plugins((
            ExtractResourcePlugin::<CaptureImage>::default(),
            ExtractResourcePlugin::<CaptureWindowTarget>::default(),
        ));
        app.add_systems(Update, sync_capture_target.before(CaptureSystems));
        app.add_observer(write_capture_png);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            override_window_attachment
                .in_set(RenderSystems::PrepareViews)
                .after(prepare_view_attachments)
                .before(prepare_view_targets),
        );
        render_app.add_systems(
            RenderGraph,
            blit_capture_to_window.in_set(RenderGraphSystems::Finish),
        );
    }
}
