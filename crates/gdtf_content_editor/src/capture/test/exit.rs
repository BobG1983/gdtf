use bevy::{app::AppExit, prelude::*, state::app::StatesPlugin};
use gdtf_screenshot::{CapturePath, CaptureQueue, PollCap, SettleFrames};

use super::super::EditorCapturePlugin;
use crate::EditorState;

const TEST_SETTLE: u32 = 1;

const TEST_POLL_BUDGET: u32 = 2;

const DRIVE_UPDATES: u32 = 32;

fn editor_capture_app(path: CapturePath) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<EditorState>();
    app.add_plugins(EditorCapturePlugin::with_path(path));
    app.insert_resource(SettleFrames::new(TEST_SETTLE));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
    app
}

#[test]
fn the_editor_exits_once_its_capture_finishes() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = editor_capture_app(CapturePath::new(tmp.path().join("editor.png")));

    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(None, ());

    let mut observed = None;
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Some(exit) = app.should_exit() {
            observed = Some(exit);
            break;
        }
    }
    assert_eq!(
        observed,
        Some(AppExit::Success),
        "GDTF_EDITOR_SHOT takes one shot and leaves; a finished capture must write AppExit or the \
         editor runs forever after writing its PNG",
    );
}

#[test]
fn the_editor_stays_up_while_its_capture_is_still_running() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = editor_capture_app(CapturePath::new(tmp.path().join("editor.png")));

    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(None, ());
    app.update();

    assert_eq!(
        app.should_exit(),
        None,
        "the exit must wait for the capture to finish — leaving on the frame it was queued would \
         kill the process before the PNG lands",
    );
}
