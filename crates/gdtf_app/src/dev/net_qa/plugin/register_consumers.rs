use std::path::PathBuf;

use bevy::prelude::*;
use gdtf_screenshot::{CapturePipelinePlugin, ShotDir};

use crate::dev::net_qa::commands::capture::screenshot::ShotResponder;

const QA_SHOT_DIR: &str = "target/qa_screenshots";

pub(super) fn register_consumers(app: &mut App) {
    app.insert_resource(ShotDir::new(PathBuf::from(QA_SHOT_DIR)));
    if !app.is_plugin_added::<CapturePipelinePlugin<ShotResponder>>() {
        app.add_plugins(CapturePipelinePlugin::<ShotResponder>::new());
    }
}
