use bevy::prelude::*;
use gdtf_qa_command::dispatch::{CaptureTicket, QaCommandSystems};
use gdtf_screenshot::{CapturePipelinePlugin, ShotDir, ShotDirName};

use crate::dev::net_qa::commands::capture::{drive_rider_captures, screenshot::ShotResponder};

const QA_SHOT_DIR: &str = "qa_screenshots";

pub(super) fn register_consumers(app: &mut App) {
    app.insert_resource(ShotDir::under_workspace_target(&ShotDirName::new(
        QA_SHOT_DIR,
    )));
    if !app.is_plugin_added::<CapturePipelinePlugin<ShotResponder>>() {
        app.add_plugins(CapturePipelinePlugin::<ShotResponder>::new());
    }
    if !app.is_plugin_added::<CapturePipelinePlugin<CaptureTicket>>() {
        app.add_plugins(CapturePipelinePlugin::<CaptureTicket>::new());
        app.add_systems(Update, drive_rider_captures.after(QaCommandSystems::Claim));
    }
}
