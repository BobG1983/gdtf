use bevy::{
    asset::Assets,
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
    window::WindowRef,
};

use super::support::{
    drive_until_settled, enqueue, install_capture_target, outcomes, pump_app,
    spawn_camera_aimed_at, spawned_captures,
};
use crate::capture::{CaptureOutcome, CaptureSource};

const FIXTURE_SCALE_FACTOR: f32 = 2.5;

fn refusal_detail(app: &App) -> Option<String> {
    match outcomes(app).first() {
        Some(CaptureOutcome::Refused(detail)) => Some(detail.as_str().to_owned()),
        _ => None,
    }
}

fn assert_names_both_targets(detail: &str, wanted: &ImageRenderTarget, rendered: &RenderTarget) {
    let wanted_text = format!("{wanted:?}");
    let rendered_text = format!("{rendered:?}");
    assert_ne!(
        wanted_text, rendered_text,
        "test setup: the two render targets must differ, or there is no mismatch to report",
    );
    assert!(
        detail.contains(&wanted_text),
        "the refusal must name the target the capture would read ({wanted_text}); it said \
         {detail:?}",
    );
    assert!(
        detail.contains(&rendered_text),
        "the refusal must name what the camera actually renders into ({rendered_text}); it said \
         {detail:?}",
    );
}

#[test]
fn a_capture_of_an_unrendered_target_is_refused_naming_both_targets() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let wanted = install_capture_target(&mut app, FIXTURE_SCALE_FACTOR);
    let rendered = RenderTarget::Window(WindowRef::Primary);
    spawn_camera_aimed_at(&mut app, rendered.clone());

    enqueue(&mut app, "unrendered");
    drive_until_settled(&mut app);

    let Some(detail) = refusal_detail(&app) else {
        unreachable!(
            "a capture of an offscreen target no camera renders into must be Refused, got {:?}",
            outcomes(&app),
        );
    };
    assert_eq!(
        spawned_captures(&mut app),
        0,
        "no Screenshot may be spawned for a refused capture — a spawned one lands a blank PNG",
    );
    assert_names_both_targets(&detail, &wanted, &rendered);
}

#[test]
fn a_camera_at_the_wrong_scale_factor_is_refused_naming_both_targets() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let wanted = install_capture_target(&mut app, FIXTURE_SCALE_FACTOR);
    let rendered = RenderTarget::Image(ImageRenderTarget::from(wanted.handle.clone()));
    spawn_camera_aimed_at(&mut app, rendered.clone());

    enqueue(&mut app, "wrong_scale");
    drive_until_settled(&mut app);

    let Some(detail) = refusal_detail(&app) else {
        unreachable!(
            "a camera aimed at the capture image at a DIFFERENT scale factor renders into a \
             different render target, so the capture must be Refused; got {:?}",
            outcomes(&app),
        );
    };
    assert_names_both_targets(&detail, &wanted, &rendered);
}

#[test]
fn a_capture_of_the_rendered_target_is_spawned() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let wanted = install_capture_target(&mut app, FIXTURE_SCALE_FACTOR);
    spawn_camera_aimed_at(&mut app, RenderTarget::Image(wanted));

    enqueue(&mut app, "rendered");
    drive_until_settled(&mut app);

    assert!(
        outcomes(&app).is_empty() && spawned_captures(&mut app) > 0,
        "with a camera aimed at the capture target the pump must spawn the capture and refuse \
         nothing; it finished {:?}",
        outcomes(&app),
    );
}

#[test]
fn a_capture_is_refused_when_the_app_has_no_camera_at_all() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let wanted = install_capture_target(&mut app, FIXTURE_SCALE_FACTOR);

    enqueue(&mut app, "no_camera");
    drive_until_settled(&mut app);

    let Some(detail) = refusal_detail(&app) else {
        unreachable!(
            "with no camera in the app nothing renders into the capture target, so the capture \
             must be Refused, got {:?}",
            outcomes(&app),
        );
    };
    assert_eq!(
        spawned_captures(&mut app),
        0,
        "no Screenshot may be spawned when nothing can have drawn into the target",
    );
    let wanted_text = format!("{wanted:?}");
    assert!(
        detail.contains(&wanted_text),
        "the refusal must still name the target the capture would read ({wanted_text}); it said \
         {detail:?}",
    );
}

#[test]
fn a_caller_pinned_source_that_is_not_the_present_target_is_not_refused() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    let present = install_capture_target(&mut app, FIXTURE_SCALE_FACTOR);
    let elsewhere = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let pinned = ImageRenderTarget::from(elsewhere);
    assert_ne!(
        pinned, present,
        "test setup: the pinned source must differ from the present path's target",
    );
    app.insert_resource(CaptureSource::Offscreen(pinned));

    enqueue(&mut app, "elsewhere");
    drive_until_settled(&mut app);

    assert!(
        outcomes(&app).is_empty() && spawned_captures(&mut app) > 0,
        "the aim check only guards the present path's own target; a caller who pinned a \
         different image owns that choice and must not be refused, it finished {:?}",
        outcomes(&app),
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
    app.insert_resource(CaptureSource::Offscreen(ImageRenderTarget::from(handle)));

    enqueue(&mut app, "no_present_path");
    drive_until_settled(&mut app);

    assert!(
        outcomes(&app).is_empty() && spawned_captures(&mut app) > 0,
        "with no present path the aim check must not refuse a caller-pinned offscreen capture; \
         it finished {:?}",
        outcomes(&app),
    );
}
