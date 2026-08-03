use bevy::{
    asset::Assets,
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
};
use bevy_egui::PrimaryEguiContext;
use gdtf_qa_protocol::{command::CommandOutcome, ids::ShotName, message::QaResponse};

use super::{
    super::config::EditorShotSource,
    support::{DRIVE_UPDATES, enqueue, pump_app, spawned_captures},
};
use crate::net_qa::present::EditorQaCaptureTarget;

const FIXTURE_SCALE_FACTOR: f32 = 2.5;

fn install_present_target(app: &mut App) -> ImageRenderTarget {
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let target = ImageRenderTarget {
        handle,
        scale_factor: FIXTURE_SCALE_FACTOR,
    };
    app.insert_resource(EditorQaCaptureTarget::new(target.clone()));
    app.insert_resource(EditorShotSource::Offscreen(target.clone()));
    target
}

fn reply_or_capture(
    app: &mut App,
    rx: &std::sync::mpsc::Receiver<QaResponse>,
) -> Option<QaResponse> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Ok(reply) = rx.try_recv() {
            return Some(reply);
        }
        if spawned_captures(app) > 0 {
            return None;
        }
    }
    None
}

#[test]
fn a_capture_of_an_unrendered_target_is_refused_with_a_message() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    install_present_target(&mut app);
    app.world_mut()
        .spawn((PrimaryEguiContext, RenderTarget::default()));

    let reply_rx = enqueue(&mut app, ShotName::new("unrendered".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        matches!(
            reply,
            Some(QaResponse::Outcome(CommandOutcome::Unavailable { .. }))
        ),
        "a capture of an offscreen target the UI camera does not render into must be refused as \
         Unavailable, never spawned and never answered with an attachment; got {reply:?}",
    );
    assert!(
        spawned_captures(&mut app) == 0,
        "no Screenshot may be spawned for a refused capture — a spawned one lands a blank PNG",
    );
    let detail = match reply {
        Some(QaResponse::Outcome(CommandOutcome::Unavailable { note, .. })) => {
            note.as_str().to_owned()
        }
        _ => String::new(),
    };
    assert!(
        detail.contains("Window") && detail.contains("Image"),
        "the refusal must name BOTH render targets so a caller can see the mismatch; it said \
         {detail:?}",
    );
}

#[test]
fn a_capture_of_the_rendered_target_is_spawned() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = install_present_target(&mut app);
    app.world_mut()
        .spawn((PrimaryEguiContext, RenderTarget::Image(target)));

    let reply_rx = enqueue(&mut app, ShotName::new("rendered".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        reply.is_none() && spawned_captures(&mut app) > 0,
        "with the UI camera aimed at the capture target the pump must spawn the capture and not \
         refuse it; it replied {reply:?}",
    );
}

#[test]
fn a_camera_at_the_wrong_scale_factor_is_refused() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let target = install_present_target(&mut app);
    app.world_mut().spawn((
        PrimaryEguiContext,
        RenderTarget::Image(ImageRenderTarget::from(target.handle)),
    ));

    let reply_rx = enqueue(&mut app, ShotName::new("wrong_scale".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        matches!(
            reply,
            Some(QaResponse::Outcome(CommandOutcome::Unavailable { .. }))
        ),
        "a UI camera aimed at the capture image at a DIFFERENT scale factor renders into a \
         different render target, so the capture must be refused; got {reply:?}",
    );
}

#[test]
fn a_caller_pinned_offscreen_source_without_a_present_path_is_still_captured() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.insert_resource(EditorShotSource::Offscreen(ImageRenderTarget::from(handle)));

    let reply_rx = enqueue(&mut app, ShotName::new("no_present_path".to_owned()));
    let reply = reply_or_capture(&mut app, &reply_rx);

    assert!(
        reply.is_none() && spawned_captures(&mut app) > 0,
        "with no present path the check must not refuse a caller-pinned offscreen capture; it \
         replied {reply:?}",
    );
}
