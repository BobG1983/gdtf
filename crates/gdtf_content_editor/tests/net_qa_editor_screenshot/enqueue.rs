use bevy::prelude::*;
use gdtf_screenshot::{CaptureCompletions, CaptureOutcome, CaptureQueue, ShotStem};

pub(crate) fn enqueue_capture(app: &mut App, name: &str) {
    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new(name)), ());
}

pub(crate) fn finished_captures(app: &App) -> Vec<CaptureOutcome> {
    app.world()
        .resource::<CaptureCompletions<()>>()
        .iter()
        .map(|completion| completion.outcome().clone())
        .collect()
}
